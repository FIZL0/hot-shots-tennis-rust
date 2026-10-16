//! P55: the match's render pipelines compiled before it starts. A cold match load compiled ~105 of them one after
//! another behind its loading screen (hst-log 10: ~15 s of a 19 s load in the browser, 1–2 s page freezes). Every render
//! pipeline the game asks for is recorded, its descriptor with the shaders by path, into `pipelines.json` beside
//! settings.txt (localStorage on the web); the next run queues them again ahead of time ([`Warmup::feed`], driven by
//! the menu: `mode.rs` `warm_up`) through the pipeline cache's `prewarm_render_pipeline` (third_party/bevy_render),
//! which hands the game's own later request for an equal descriptor that pipeline's id, compiled or on its way.
//! A record from another GPU or backend is dropped; one no match asked for in [`KEEP`] matches is left out on save.
//! `HST_PREWARM=0` turns it off (neither recorded nor warmed).

use bevy::platform::time::Instant;
use bevy::prelude::*;
use bevy::render::render_resource::{
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, CachedPipelineState, CachedRenderPipelineId, ColorTargetState, DepthStencilState, FragmentState, MultisampleState, PipelineCache,
    PipelineDescriptor, PrimitiveState, RenderPipelineDescriptor, VertexAttribute, VertexState, VertexStepMode,
};
use bevy::mesh::VertexBufferLayout;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::render::{Render, RenderApp, RenderSystems as R};
use bevy::shader::{Shader, ShaderDefVal};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Where pipelines compile on the render thread, one at a time inside the frame (the browser, macOS: `PipelineCache`'s
/// `compiles_in_place`): there the menu feeds the record a pipeline a frame while nobody touches the controls.
/// Elsewhere they compile on the async pool and the whole record goes at once.
pub const IN_PLACE: bool = cfg!(any(target_arch = "wasm32", target_os = "macos"));

/// Matches a recorded pipeline is kept for without one asking for it.
const KEEP: u32 = 8;

/// The most pipelines the record holds (the newest kept).
const MAX: usize = 1000;

/// The record's format; another one is dropped.
const VERSION: u32 = 1;

/// The quiet after the last newly asked-for pipeline before the record is saved.
const SAVE_AFTER: std::time::Duration = std::time::Duration::from_secs(2);

/// `Match`: the match's resource (`play::Game`): pipelines asked for while it's on count as a match's.
pub fn plugin<Match: Resource>(app: &mut App) {
    if std::env::var("HST_PREWARM").is_ok_and(|v| v == "0") {
        return;
    }
    let link = Link::default();
    let Some(r) = app.get_sub_app_mut(RenderApp) else { return };
    r.insert_resource(link.clone()).add_systems(Render, (prewarm.in_set(R::Prepare), report.in_set(R::Cleanup)));
    app.insert_resource(Warmup { link, ..default() }).add_systems(Last, collect::<Match>);
}

/// Descriptors between the worlds: to pre-warm (main → render) and newly asked for (render → main).
#[derive(Resource, Clone, Default)]
struct Link {
    warm: Arc<Mutex<Vec<RenderPipelineDescriptor>>>,
    asked: Arc<Mutex<Vec<RenderPipelineDescriptor>>>,
}

/// The record and how far the menu fed it to the pipeline cache.
#[derive(Resource, Default)]
pub struct Warmup {
    link: Link,
    /// Read from the store (once the renderer and `Args` are up).
    loaded: bool,
    path: std::path::PathBuf,
    record: Record,
    /// The record's entries from the last run still to feed, in order.
    todo: std::collections::VecDeque<usize>,
    /// A match asked for a pipeline this run: `record.matches` counted it.
    counted: bool,
    /// Newly asked-for pipelines not saved yet, since this.
    dirty: Option<Instant>,
    /// Recorded pipelines left out: a shader not loaded.
    skipped: usize,
    /// Shader paths → this run's shaders (built when a path doesn't resolve).
    shaders: HashMap<String, AssetId<Shader>>,
}

impl Warmup {
    /// Recorded pipelines not fed to the pipeline cache yet.
    pub fn pending(&self) -> usize {
        self.todo.len()
    }

    /// Queue the next `n` recorded pipelines to compile (all left: `usize::MAX`). One whose shader isn't loaded (yet)
    /// is skipped; it compiles when the game asks for it.
    pub fn feed(&mut self, server: &AssetServer, shaders: &mut Assets<Shader>, n: usize) {
        let mut out = Vec::new();
        while out.len() < n
            && let Some(i) = self.todo.pop_front()
        {
            match self.record.descriptor(&self.record.pipelines[i].0, server, shaders, &mut self.shaders) {
                Some(d) => out.push(d),
                None => self.skipped += 1,
            }
        }
        if self.todo.is_empty() && !out.is_empty() {
            info!("pipelines: the record fed to the pipeline cache ({} skipped: a shader not loaded) at {:.2} s", self.skipped, crate::stall::since_start());
        }
        if !out.is_empty() {
            self.link.warm.lock().unwrap().extend(out);
        }
    }
}

/// `pipelines.json`: the GPU it's for, the matches played, and the pipelines with their layouts, shader paths and
/// shader def lists interned.
#[derive(Serialize, Deserialize, Default)]
struct Record {
    version: u32,
    device: String,
    /// Matches that asked for a pipeline (the age [`KEEP`] counts in).
    matches: u32,
    shaders: Vec<String>,
    layouts: Vec<Layout>,
    defs: Vec<Vec<ShaderDefVal>>,
    /// Each with the match count when one last asked for it.
    pipelines: Vec<(Pipeline, u32)>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
struct Layout {
    label: String,
    entries: Vec<BindGroupLayoutEntry>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
struct Stage {
    shader: u32,
    defs: u32,
    entry: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
struct Buffer {
    stride: u64,
    step: VertexStepMode,
    attributes: Vec<VertexAttribute>,
}

/// One render pipeline's descriptor.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
struct Pipeline {
    label: Option<String>,
    layout: Vec<u32>,
    immediate: u32,
    vertex: Stage,
    buffers: Vec<Buffer>,
    primitive: PrimitiveState,
    depth: Option<DepthStencilState>,
    multisample: MultisampleState,
    fragment: Option<(Stage, Vec<Option<ColorTargetState>>)>,
    zero_init: bool,
}

/// `v`'s index in `table`, added if new.
fn intern<T: PartialEq>(table: &mut Vec<T>, v: T) -> u32 {
    (table.iter().position(|t| *t == v).unwrap_or_else(|| {
        table.push(v);
        table.len() - 1
    })) as u32
}

impl Record {
    /// `d` as a record entry; `None` when a shader has no path.
    fn entry(&mut self, d: &RenderPipelineDescriptor, shaders: &Assets<Shader>) -> Option<Pipeline> {
        let mut stage = |shader: &Handle<Shader>, defs: &[ShaderDefVal], entry: &Option<std::borrow::Cow<'static, str>>| {
            let path = shaders.get(shader.id()).map(|s| s.path.clone()).filter(|p| !p.is_empty())?;
            Some(Stage { shader: intern(&mut self.shaders, path), defs: intern(&mut self.defs, defs.to_vec()), entry: entry.as_ref().map(|e| e.to_string()) })
        };
        let vertex = stage(&d.vertex.shader, &d.vertex.shader_defs, &d.vertex.entry_point)?;
        let fragment = match &d.fragment {
            Some(f) => Some((stage(&f.shader, &f.shader_defs, &f.entry_point)?, f.targets.clone())),
            None => None,
        };
        Some(Pipeline {
            label: d.label.as_ref().map(|l| l.to_string()),
            layout: d.layout.iter().map(|l| intern(&mut self.layouts, Layout { label: l.label.to_string(), entries: l.entries.clone() })).collect(),
            immediate: d.immediate_size,
            vertex,
            buffers: d.vertex.buffers.iter().map(|b| Buffer { stride: b.array_stride, step: b.step_mode, attributes: b.attributes.clone() }).collect(),
            primitive: d.primitive,
            depth: d.depth_stencil.clone(),
            multisample: d.multisample,
            fragment,
            zero_init: d.zero_initialize_workgroup_memory,
        })
    }

    /// Entry `p` as this run's descriptor; `None` when one of its shaders isn't loaded.
    fn descriptor(&self, p: &Pipeline, server: &AssetServer, shaders: &mut Assets<Shader>, ids: &mut HashMap<String, AssetId<Shader>>) -> Option<RenderPipelineDescriptor> {
        let mut handle = |s: &Stage| {
            let path = self.shaders.get(s.shader as usize)?;
            // an asset path (`embedded://…`) through the asset server, which hands the game's own load of it this
            // handle's asset while the pre-warmed pipeline holds it (one only `Assets` holds, it would load anew)
            if path.contains("://") {
                return Some(server.get_handle(path.clone()).unwrap_or_else(|| server.load(path.clone())));
            }
            if !ids.contains_key(path) {
                // by path, a path two shaders share left out
                ids.clear();
                let mut seen = std::collections::HashSet::new();
                let mut shared = std::collections::HashSet::new();
                for (id, s) in shaders.iter() {
                    if !seen.insert(s.path.clone()) {
                        shared.insert(s.path.clone());
                    }
                    ids.insert(s.path.clone(), id);
                }
                ids.retain(|p, _| !shared.contains(p));
            }
            shaders.get_strong_handle(*ids.get(path)?)
        };
        let vertex_shader = handle(&p.vertex)?;
        let fragment = match &p.fragment {
            Some((s, targets)) => Some(FragmentState {
                shader: handle(s)?,
                shader_defs: self.defs.get(s.defs as usize)?.clone(),
                entry_point: s.entry.clone().map(Into::into),
                targets: targets.clone(),
            }),
            None => None,
        };
        let layout = p.layout.iter().map(|&i| self.layouts.get(i as usize).map(|l| BindGroupLayoutDescriptor { label: l.label.clone().into(), entries: l.entries.clone() })).collect::<Option<_>>()?;
        Some(RenderPipelineDescriptor {
            label: p.label.clone().map(Into::into),
            layout,
            immediate_size: p.immediate,
            vertex: VertexState {
                shader: vertex_shader,
                shader_defs: self.defs.get(p.vertex.defs as usize)?.clone(),
                entry_point: p.vertex.entry.clone().map(Into::into),
                buffers: p.buffers.iter().map(|b| VertexBufferLayout { array_stride: b.stride, step_mode: b.step, attributes: b.attributes.clone() }).collect(),
            },
            primitive: p.primitive,
            depth_stencil: p.depth.clone(),
            multisample: p.multisample,
            fragment,
            zero_initialize_workgroup_memory: p.zero_init,
        })
    }

    /// The record to save: the entries a match asked for within [`KEEP`] matches (the newest [`MAX`]), tables rebuilt.
    fn pruned(&self) -> Record {
        let mut keep: Vec<&(Pipeline, u32)> = self.pipelines.iter().filter(|p| p.1 + KEEP > self.matches).collect();
        if keep.len() > MAX {
            keep.sort_by_key(|p| std::cmp::Reverse(p.1));
            keep.truncate(MAX);
        }
        let mut out = Record { version: VERSION, device: self.device.clone(), matches: self.matches, ..default() };
        for (p, last) in keep {
            let mut p = p.clone();
            let stage = |s: &mut Stage, out: &mut Record| {
                s.shader = intern(&mut out.shaders, self.shaders[s.shader as usize].clone());
                s.defs = intern(&mut out.defs, self.defs[s.defs as usize].clone());
            };
            stage(&mut p.vertex, &mut out);
            if let Some((s, _)) = &mut p.fragment {
                stage(s, &mut out);
            }
            for l in &mut p.layout {
                *l = intern(&mut out.layouts, self.layouts[*l as usize].clone());
            }
            out.pipelines.push((p, *last));
        }
        out
    }
}

/// The GPU and backend a record is for.
fn device(info: &RenderAdapterInfo) -> String {
    format!("{} | {:?} | {}", info.name, info.backend, info.driver_info)
}

/// Main world: loads the record once the renderer is up, adds the pipelines the game newly asked for and saves it once
/// they stop coming.
fn collect<Match: Resource>(mut w: ResMut<Warmup>, args: Option<Res<crate::Args>>, info: Option<Res<RenderAdapterInfo>>, shaders: Res<Assets<Shader>>, game: Option<Res<Match>>) {
    let w = &mut *w;
    if !w.loaded {
        let (Some(args), Some(info)) = (args, info) else { return };
        w.loaded = true;
        w.path = crate::store::settings(&args.iso).with_file_name("pipelines.json");
        let device = device(&info);
        w.record = crate::store::read_to_string(&w.path)
            .ok()
            .and_then(|t| serde_json::from_str::<Record>(&t).ok())
            .filter(|r| r.version == VERSION && r.device == device)
            .unwrap_or(Record { version: VERSION, device, ..default() });
        w.todo = (0..w.record.pipelines.len()).collect();
        info!("pipelines: {} recorded for this GPU", w.todo.len());
    }
    let asked = std::mem::take(&mut *w.link.asked.lock().unwrap());
    if !asked.is_empty() {
        if game.is_some() && !std::mem::replace(&mut w.counted, true) {
            w.record.matches += 1;
        }
        let last = w.record.matches;
        for d in &asked {
            let Some(p) = w.record.entry(d, &shaders) else { continue };
            match w.record.pipelines.iter_mut().find(|q| q.0 == p) {
                Some(q) => q.1 = last,
                None => w.record.pipelines.push((p, last)),
            }
        }
        w.dirty = Some(Instant::now());
    }
    if w.dirty.is_some_and(|t| t.elapsed() >= SAVE_AFTER) {
        w.dirty = None;
        let record = w.record.pruned();
        match serde_json::to_string(&record) {
            Ok(text) => {
                if let Err(e) = crate::store::write(&w.path, &text) {
                    warn!("pipelines: couldn't save {}: {e}", w.path.display());
                }
            }
            Err(e) => warn!("pipelines: {e}"),
        }
    }
}

/// Render world: the menu's pre-warm descriptors into the pipeline cache.
fn prewarm(link: Res<Link>, cache: Res<PipelineCache>) {
    for d in std::mem::take(&mut *link.warm.lock().unwrap()) {
        let _: CachedRenderPipelineId = cache.prewarm_render_pipeline(d);
    }
}

/// Render world: each render pipeline the game asked for, once it's built, to the main world's record.
fn report(link: Res<Link>, cache: Res<PipelineCache>, mut done: Local<Vec<bool>>) {
    let mut asked = Vec::new();
    let n = cache.pipelines().count();
    if done.len() < n {
        done.resize(n, false);
    }
    for (id, p) in cache.pipelines().enumerate() {
        if done[id] {
            continue;
        }
        let PipelineDescriptor::RenderPipelineDescriptor(d) = &p.descriptor else {
            done[id] = true;
            continue;
        };
        if matches!(p.state, CachedPipelineState::Ok(_)) && cache.requested(id) {
            asked.push((**d).clone());
            done[id] = true;
        }
    }
    if !asked.is_empty() {
        link.asked.lock().unwrap().extend(asked);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::render_resource::{BindingType, BufferBindingType, ShaderStages, TextureFormat, VertexFormat};

    fn desc(shader: &Handle<Shader>, label: &str, def: i32) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some(label.to_string().into()),
            layout: vec![BindGroupLayoutDescriptor {
                label: "view".into(),
                entries: vec![BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: None },
                    count: None,
                }],
            }],
            vertex: VertexState {
                shader: shader.clone(),
                shader_defs: vec!["SKINNED".into(), ShaderDefVal::Int("LAYER".into(), def)],
                buffers: vec![VertexBufferLayout::from_vertex_formats(VertexStepMode::Vertex, [VertexFormat::Float32x3, VertexFormat::Float32x2])],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: shader.clone(),
                shader_defs: vec!["SKINNED".into()],
                targets: vec![Some(TextureFormat::Rgba8UnormSrgb.into())],
                ..default()
            }),
            ..default()
        }
    }

    /// A descriptor goes into the record, through the JSON and back out equal; one no match asked for in [`KEEP`]
    /// matches is left out of the saved record, with the tables only it used.
    #[test]
    fn a_recorded_pipeline_comes_back_equal() {
        let mut app = App::new();
        app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default())).init_asset::<Shader>();
        let w = app.world_mut();
        let server = w.resource::<AssetServer>().clone();
        let mut shaders = w.remove_resource::<Assets<Shader>>().unwrap();
        let shader = shaders.add(Shader::from_wgsl("@vertex fn vertex() {}", "test/pipelines.wgsl"));
        let mut record = Record { version: VERSION, device: "test".into(), ..default() };
        let (old, new) = (desc(&shader, "old", 1), desc(&shader, "new", 2));
        let p = record.entry(&old, &shaders).unwrap();
        record.pipelines.push((p, 0));
        record.matches = KEEP;
        let p = record.entry(&new, &shaders).unwrap();
        record.pipelines.push((p, KEEP));
        assert_eq!(record.defs.len(), 3);
        let saved: Record = serde_json::from_str(&serde_json::to_string(&record.pruned()).unwrap()).unwrap();
        assert_eq!(saved.pipelines.len(), 1, "the old one dropped");
        assert_eq!(saved.defs.len(), 2, "its LAYER=1 defs with it");
        let back = saved.descriptor(&saved.pipelines[0].0, &server, &mut shaders, &mut HashMap::new()).unwrap();
        assert!(back == new, "the same descriptor back");
    }
}
