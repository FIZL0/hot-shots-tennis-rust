//! B357: a mesh whose vertices move every frame without its asset changing. Each `Assets<Mesh>::get_mut` makes
//! Bevy re-extract the whole mesh, free and reallocate its slab space, repack every attribute and rewrite its
//! vertices and indices, and rebuild what pointed at the old allocation: the costume sway's dozen meshes (≈ 3,600
//! vertices) did that every frame, the browser build's biggest per-frame cost after the draws and a source of its
//! frame-time spikes. A [`Patched`] mesh instead keeps its asset as built and hands the render world the bytes of the
//! vertices that moved, which it writes into the mesh's allocation in the vertex buffer as it stands.

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy::render::mesh::allocator::MeshAllocator;
use bevy::render::renderer::RenderQueue;
use bevy::render::{ExtractSchedule, MainWorld, Render, RenderApp, RenderSystems};

/// A mesh's vertex data packed as Bevy uploads it (each vertex's attributes in attribute-id order, interleaved), kept
/// in the main world so moved positions can be written over it.
pub struct Patched {
    mesh: AssetId<Mesh>,
    packed: Vec<u8>,
    stride: usize,
    /// The position's byte offset in a vertex.
    at: usize,
}

/// The writes queued this frame (P40: their bytes in one buffer, not an allocation each).
#[derive(Resource, Default)]
pub struct Queued {
    writes: Vec<Write>,
    bytes: Vec<u8>,
}

/// A write: the mesh, its vertex size, the first vertex written, and where its bytes are in [`Queued::bytes`].
struct Write {
    mesh: AssetId<Mesh>,
    stride: u32,
    first: u32,
    at: usize,
    len: usize,
}

impl Queued {
    fn push(&mut self, mesh: AssetId<Mesh>, stride: u32, first: u32, bytes: &[u8]) {
        self.writes.push(Write { mesh, stride, first, at: self.bytes.len(), len: bytes.len() });
        self.bytes.extend_from_slice(bytes);
    }

    /// Moves `other`'s writes after these.
    fn append(&mut self, other: &mut Queued) {
        let base = self.bytes.len();
        self.writes.extend(other.writes.drain(..).map(|w| Write { at: w.at + base, ..w }));
        self.bytes.append(&mut other.bytes);
    }

    fn clear(&mut self) {
        self.writes.clear();
        self.bytes.clear();
    }
}

/// The most bytes of unmoved vertices `send` rewrites to join two runs into one write: copying a couple of KB costs
/// less than another call into the GPU queue (a staging copy and a copy command each; in the browser a WebGPU call).
const JOIN: usize = 2048;

pub fn plugin(app: &mut App) {
    app.init_resource::<Queued>();
    if let Some(r) = app.get_sub_app_mut(RenderApp) {
        r.init_resource::<Queued>()
            .add_systems(ExtractSchedule, extract)
            .add_systems(Render, write.in_set(RenderSystems::PrepareResources));
    }
}

impl Patched {
    /// `mesh` as its vertex buffer holds it; None if it has no `Float32x3` positions. Its asset is set by `of`.
    pub fn new(mesh: &Mesh) -> Option<Patched> {
        let stride = mesh.get_vertex_size() as usize;
        let n = mesh.count_vertices();
        let mut packed = vec![0u8; stride * n];
        let (mut offset, mut at) = (0, None);
        for (attr, values) in mesh.attributes() {
            let size = attr.format.size() as usize;
            if attr.id == Mesh::ATTRIBUTE_POSITION.id {
                matches!(values, VertexAttributeValues::Float32x3(_)).then_some(())?;
                at = Some(offset);
            }
            for (v, b) in values.get_bytes().chunks_exact(size).take(n).enumerate() {
                packed[v * stride + offset..][..size].copy_from_slice(b);
            }
            offset += size;
        }
        Some(Patched { mesh: AssetId::default(), packed, stride, at: at? })
    }

    /// The same vertices, for the mesh asset `id`.
    pub fn of(self, id: AssetId<Mesh>) -> Patched {
        Patched { mesh: id, ..self }
    }

    /// Set vertex `v`'s position; true if it changed.
    pub fn set(&mut self, v: usize, p: [f32; 3]) -> bool {
        let Some(dst) = self.packed.get_mut(v * self.stride + self.at..).and_then(|d| d.get_mut(..12)) else { return false };
        let mut new = [0u8; 12];
        for (d, x) in new.as_chunks_mut::<4>().0.iter_mut().zip(p) {
            d.copy_from_slice(&x.to_ne_bytes());
        }
        if dst == new {
            return false;
        }
        dst.copy_from_slice(&new);
        true
    }

    /// Queue vertices `lo..=hi` (as `set` left them) for the GPU.
    fn queue(&self, lo: usize, hi: usize, q: &mut Queued) {
        q.push(self.mesh, self.stride as u32, lo as u32, &self.packed[lo * self.stride..(hi + 1) * self.stride]);
    }
}

/// Queues the runs of `changed` vertices (sorted) of `p`; vertices less than a few apart (up to [`JOIN`] bytes between
/// them) go in one write.
pub fn send(p: &Patched, changed: &[usize], queued: &mut Queued) {
    let gap = (JOIN / p.stride.max(1)).max(16);
    let mut run: Option<(usize, usize)> = None;
    for &v in changed {
        run = match run {
            Some((lo, hi)) if v <= hi + gap => Some((lo, v)),
            Some((lo, hi)) => {
                p.queue(lo, hi, queued);
                Some((v, v))
            }
            None => Some((v, v)),
        };
    }
    if let Some((lo, hi)) = run {
        p.queue(lo, hi, queued);
    }
}

/// The main world's queued writes, moved to the render world.
fn extract(mut main: ResMut<MainWorld>, mut queued: ResMut<Queued>) {
    queued.append(&mut main.resource_mut::<Queued>());
}

/// Each write into its mesh's place in the vertex buffer (after this frame's allocations). A mesh not allocated yet
/// keeps its writes for the next frames (a while: a despawned rig's mesh never will be), ahead of the newer ones.
fn write(mut queued: ResMut<Queued>, mut waiting: Local<(Vec<u8>, Queued)>, mut kept: Local<(Vec<u8>, Queued)>, allocator: Res<MeshAllocator>, queue: Res<RenderQueue>) {
    if queued.writes.is_empty() && waiting.1.writes.is_empty() {
        return;
    }
    let (ages, wait) = &mut *waiting;
    let (kept_ages, keep) = &mut *kept;
    kept_ages.clear();
    keep.clear();
    let new = queued.writes.iter().map(|w| (0, w, &queued.bytes));
    for (age, w, bytes) in ages.iter().copied().zip(&wait.writes).map(|(a, w)| (a, w, &wait.bytes)).chain(new) {
        let bytes = &bytes[w.at..][..w.len];
        let Some(slice) = allocator.mesh_vertex_slice(&w.mesh) else {
            if age < 60 {
                kept_ages.push(age + 1);
                keep.push(w.mesh, w.stride, w.first, bytes);
            }
            continue;
        };
        if (w.first as usize * w.stride as usize + bytes.len()) > slice.range.len() * w.stride as usize {
            continue;
        }
        queue.write_buffer(slice.buffer, (slice.range.start + w.first) as u64 * w.stride as u64, bytes);
    }
    std::mem::swap(&mut *waiting, &mut *kept);
    queued.clear();
}
