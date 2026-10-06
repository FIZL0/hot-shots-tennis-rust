//! Collision data read from the disc against the game's own copy in RAM (`context/ram/s05.bin`, a dump of
//! save-state slot 5 on court 10; not in git, skipped when absent along with the disc image). Every collision
//! object the live ball queries — the court model and each prop in the world grid (net, umpire chair, trees, …) —
//! is matched to its `.mdl` on the disc by content (the file is loaded in place), then the game's triangle walk (node tree pre-order, the
//! per-node material/batch lists it builds at load, packets, listed strip slots minus restarts) is replayed over
//! the RAM structures and compared bit for bit with `mdl::parse`'s collision list, along with node matrices,
//! two-sided flags and attribute maps.

use hst_data::iso::Iso;
use hst_data::mdl::{self, CollisionTri};
use hst_data::mtl;
use hst_data::xb::Archive;

const GM: u32 = 0x422f80;
const GRID: u32 = 0x4238a8;
const GRID_BOUNDS: u32 = 0x4238e0;
const GRID_DIMS: (u32, u32) = (0x423918, 0x423920);

struct Ram(Vec<u8>);
impl Ram {
    fn b(&self, a: u32) -> u8 {
        self.0[(a & 0x1ff_ffff) as usize]
    }
    fn bytes(&self, a: u32, n: usize) -> &[u8] {
        let a = (a & 0x1ff_ffff) as usize;
        &self.0[a..a + n]
    }
    fn u(&self, a: u32) -> u32 {
        u32::from_le_bytes(self.bytes(a, 4).try_into().unwrap())
    }
    fn i(&self, a: u32) -> i32 {
        self.u(a) as i32
    }
    fn f(&self, a: u32) -> f32 {
        f32::from_bits(self.u(a))
    }
    fn qword(&self, a: u32) -> [f32; 4] {
        std::array::from_fn(|k| self.f(a + 4 * k as u32))
    }
}

/// The game's triangle walk over one model instance (`inst` = model instance, `node` = node instance).
fn walk(r: &Ram, data: u32, node: u32, out: &mut Vec<CollisionTri>, mats: &mut Vec<(usize, [[f32; 4]; 4])>) {
    let def = r.u(node + 0x108);
    let idx = r.u(node + 0x10c) as usize;
    let m = r.u(def + 0x10) + 0x80;
    mats.push((idx, std::array::from_fn(|row| r.qword(m + 16 * row as u32))));
    let entry = r.u(r.u(data + 0x30)) + idx as u32 * 0xc;
    for e in 0..r.u(entry + 4) {
        let pair = r.u(entry + 8) + e * 4;
        let (a, b) = (r.b(pair) as u32, r.b(pair + 1) as u32);
        let batch = r.u(r.u(r.u(r.u(r.u(data + 0x1c) + 8) + a * 4) + 8) + b * 4);
        for p in 0..r.u(r.u(batch + 4) + 0x1c) {
            let pk = r.u(r.u(batch + 0x14) + p * 4);
            let (h, vif, map, starts, bounds) = (r.u(pk + 4), r.u(pk + 0xc), r.u(pk + 0x10), r.u(pk + 0x14), r.u(pk + 0x18));
            let (pos, nrm, col, uv) = (r.u(h + 0x44), r.u(h + 0x48), r.i(h + 0x4c), r.u(h + 0x50));
            for i in 0..r.u(h + 0x38) {
                let s = r.b(starts + i) as u32;
                let slot = |k: u32| r.b(map + s + k) as u32;
                if r.u(vif + nrm * 16 + slot(2) * 8 + 4) & 0x8000_0000 != 0 {
                    continue;
                }
                out.push(CollisionTri {
                    node: idx,
                    material: a as usize,
                    pos: std::array::from_fn(|k| r.qword(vif + pos * 16 + slot(k as u32) * 16)),
                    uv: std::array::from_fn(|k| r.qword(vif + uv * 16 + (s + k as u32) * 16)),
                    color: std::array::from_fn(|k| {
                        let a = if col < 0 { pk + 0x24 } else { vif + col as u32 * 16 + (s + k as u32) * 4 };
                        r.bytes(a, 4).try_into().unwrap()
                    }),
                    flags: std::array::from_fn(|k| r.b(vif + nrm * 16 + slot(k as u32) * 8 + 6) >> 3),
                    bounds: [r.qword(bounds + i * 32), r.qword(bounds + i * 32 + 16)],
                });
            }
        }
    }
    for c in 0..r.u(def + 0x1c) {
        walk(r, data, r.u(r.u(node + 0x104) + c * 4), out, mats);
    }
}

/// Collision objects the live ball queries: the court object, then every distinct world-grid object.
fn objects(r: &Ram) -> Vec<u32> {
    let world = r.u(r.u(GM) + 0x84);
    let mut v = vec![r.u(r.u(r.u(world + 0x138)))];
    let lo: Vec<i32> = (0..6).map(|k| r.i(GRID_BOUNDS + 8 * k)).collect();
    let (d1, d2) = (r.i(GRID_DIMS.0), r.i(GRID_DIMS.1));
    for a in 0..=lo[1] - lo[0] {
        for b in 0..=lo[3] - lo[2] {
            for c in 0..=lo[5] - lo[4] {
                let cell = r.u(GRID) + ((d2 * (b + a * d1) + c) * 8) as u32;
                for k in 0..r.i(cell).max(0) as u32 {
                    let o = r.u(r.u(r.u(r.u(r.u(cell + 4) + 4 * k))));
                    if !v.contains(&o) {
                        v.push(o);
                    }
                }
            }
        }
    }
    v
}

#[test]
fn court_10_collision_matches_ram() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(ram), Ok(mut iso)) = (std::fs::read(format!("{root}/context/ram/s05.bin")), Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")))
    else {
        eprintln!("RAM dump or disc missing, skipped");
        return;
    };
    let r = Ram(ram);
    let mut files = Vec::new();
    for arc in ["GRD01.XB", "HOL01.XB"] {
        let data = iso.read(&format!("COURT/10/{arc}")).unwrap();
        let a = Archive::parse(&data).unwrap();
        let read = |n: &str| a.entries.iter().find(|e| e.name.eq_ignore_ascii_case(n)).map(|e| a.read(e).unwrap());
        for e in a.entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".mdl")) {
            let stem = &e.name[..e.name.len() - 4];
            let mtl = read(&format!("{stem}.MTL")).expect("material file");
            files.push((e.name.clone(), a.read(e).unwrap(), mtl, read(&format!("{stem}.MTI"))));
        }
    }
    let (mut models, mut tris) = (Vec::new(), 0);
    for obj in objects(&r) {
        let inst = r.u(obj + 0xc);
        let data = r.u(inst + 0x54);
        let (base, size) = (r.u(data), r.u(data + 4) as usize);
        // loading patches some bytes outside the collision data (batch state): take the closest file of that size
        let diff = |f: &[u8]| f.iter().zip(r.bytes(base, size)).filter(|(a, b)| a != b).count();
        let (name, bytes, mtl_bytes, mti) = files
            .iter()
            .filter(|f| f.1.len() == size)
            .min_by_key(|f| diff(&f.1))
            .unwrap_or_else(|| panic!("object {obj:#x}: no disc model"));
        if models.contains(name) {
            continue;
        }
        let m = mdl::parse(bytes).unwrap();
        let mt = mtl::parse(mtl_bytes, mti.as_deref()).unwrap();
        let (mut got, mut mats) = (Vec::new(), Vec::new());
        walk(&r, data, r.u(inst + 0x64), &mut got, &mut mats);
        let want: Vec<_> = m.colliding(&mt).collect();
        assert_eq!(want.len(), got.len(), "{name}: triangle count");
        for (k, (a, b)) in want.iter().zip(&got).enumerate() {
            assert_eq!(*a, b, "{name}: triangle {k}");
        }
        for (idx, mat) in mats {
            assert_eq!(m.node_inverse[idx], mat, "{name}: node {idx} matrix");
        }
        for (k, mat) in mt.materials.iter().enumerate() {
            let rec = r.u(data + 0x40) + k as u32 * 0x60;
            let state = r.u(rec + 0x10) & 0x1ff_ffff;
            assert_eq!(r.u(state + 0x10) & 1 != 0, mat.two_sided, "{name}: material {k} sides");
            assert_eq!(r.u(state + 0x10) & 8 != 0, mat.texture.is_some(), "{name}: material {k} textured");
            let clamp = r.b(state + 0x30);
            assert_eq!([clamp & 3, clamp >> 2 & 3], m.wrap[k], "{name}: material {k} wrap");
            let map = r.u(r.u(rec + 0x58) + 0x94);
            assert_eq!(map != 0, mat.attributes.is_some(), "{name}: material {k} attribute map");
            let Some(a) = mat.attributes else { continue };
            let a = &mt.attributes[a];
            assert_eq!(r.bytes(r.u(map + 4), 0x48), a.header, "{name}: material {k} map header");
            assert_eq!(r.bytes(r.u(map + 0xc), a.texels.len()), a.texels, "{name}: material {k} texels");
            // the game marks palette entries the texels never use as 0xff; only used ones are read
            let table = r.bytes(r.u(map + 0x2c), 16);
            for t in a.texels.iter().flat_map(|&b| [b & 0xf, b >> 4]) {
                assert_eq!(table[t as usize], a.table[t as usize], "{name}: material {k} table entry {t}");
            }
        }
        eprintln!("{name}: {} triangles, {} nodes", got.len(), m.node_count);
        tris += got.len();
        models.push(name.clone());
    }
    eprintln!("{} models, {tris} triangles", models.len());
    assert!(models.iter().any(|n| n.to_ascii_lowercase().contains("znet")), "net model among the collision objects");
}
