//! Collision world placement from the disc against the game's own objects in RAM (`context/ram/s05.bin`, save-state
//! slot 5 on court 10; not in git, skipped when absent along with the disc image). The plant records that become
//! props (trees, props, structures with a model and a non-zero scale) are instanced with `world::place` /
//! `world::instance` and walked in the game's list order; each must match its RAM object bit for bit: model,
//! net-post flag, scale, world matrix, root node matrices, bounding sphere centre and — for collision models — the
//! world → model matrix. The 20 m grid built from the collision props must match the game's bounds and cell lists.

use hst_data::iso::Iso;
use hst_data::xb::Archive;
use hst_data::{layout, mdl, mtl};
use hst_sim::world::{self, M4};

const LIST: u32 = 0x423928;
const GRID: u32 = 0x4238a8;
const GRID_MINMAX: u32 = 0x4238c0;
const GRID_BOUNDS: u32 = 0x4238e0;
const GRID_DIMS: u32 = 0x423910;

struct Ram(Vec<u8>);
impl Ram {
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
    fn m4(&self, a: u32) -> M4 {
        std::array::from_fn(|r| std::array::from_fn(|k| self.f(a + 16 * r as u32 + 4 * k as u32)))
    }
}

fn bits<const N: usize>(v: &[[f32; 4]; N]) -> Vec<u32> {
    v.iter().flatten().map(|x| x.to_bits()).collect()
}

#[test]
fn court_10_props_and_grid_match_ram() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(ram), Ok(mut iso)) = (std::fs::read(format!("{root}/context/ram/s05.bin")), Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")))
    else {
        eprintln!("RAM dump or disc missing, skipped");
        return;
    };
    let r = Ram(ram);
    let mut files = std::collections::HashMap::new();
    let (mut list, mut plants) = (Vec::new(), Vec::new());
    for arc in ["CMN.XB", "GRD01.XB", "HOL01.XB"] {
        let data = iso.read(&format!("COURT/10/{arc}")).unwrap();
        let a = Archive::parse(&data).unwrap();
        let read = |n: &str| a.entries.iter().find(|e| e.name.eq_ignore_ascii_case(n)).map(|e| a.read(e).unwrap());
        for e in &a.entries {
            let name = e.name.to_ascii_lowercase();
            if name.ends_with("entry_c10.txt") {
                list = layout::entries(&String::from_utf8_lossy(&a.read(e).unwrap()));
            } else if name.ends_with("plant_c10_h01_0.dat") {
                plants = layout::plants(&a.read(e).unwrap()).unwrap();
            } else if let Some(stem) = name.strip_suffix(".mdl") {
                let base = &e.name[..e.name.len() - 4];
                let m = mdl::parse(&a.read(e).unwrap()).unwrap();
                let mt = mtl::parse(&read(&format!("{base}.MTL")).unwrap(), read(&format!("{base}.MTI")).as_deref()).unwrap();
                let size = a.read(e).unwrap().len();
                files.insert(stem.rsplit(['/', '\\']).next().unwrap().to_string(), (m, mt, size));
            }
        }
    }

    // props in creation order: the game walks categories in ascending order, records in file order
    let mut created = Vec::new();
    for cat in [15, 17, 18, 19, 20] {
        for p in plants.iter().filter(|p| p.category == cat && p.scale != 0.0) {
            let Some(e) = layout::resolve(&list, p, 0) else { continue };
            let Some((model, mt, size)) = files.get(&e.stem) else { panic!("{}: no model on disc ({:?})", e.stem, files.keys().collect::<Vec<_>>()) };
            let (m, at_origin) = world::place(p.category, p.pos, p.yaw, p.code);
            created.push((e.stem.clone(), world::instance(m, p.scale, &model.node_local[0], model.center, model.radius), p.scale, at_origin, model.collides(mt), *size));
        }
    }

    let mut node = r.u(LIST);
    let mut ids = Vec::new(); // RAM list node of each prop, in list order
    let mut colliding = Vec::new();
    for k in world::list_order(created.len()) {
        let (stem, prop, scale, at_origin, collides, size) = &created[k];
        assert_ne!(node, 0, "RAM list shorter than {} props", created.len());
        let obj = r.u(r.u(r.u(node)));
        let inst = r.u(obj + 0xc);
        let data = r.u(inst + 0x54);
        let n0 = r.u(inst + 0x64);
        let at = format!("prop {} ({stem})", ids.len());
        assert_eq!(r.u(data + 4) as usize, *size, "{at}: model size");
        assert_eq!(r.bytes(node + 7, 1)[0] != 0, *at_origin, "{at}: net-post flag");
        assert_eq!(r.f(obj + 0x120).to_bits(), scale.to_bits(), "{at}: scale");
        assert_eq!(bits(&r.m4(obj + 0xe0)), bits(&prop.world), "{at}: world matrix");
        assert_eq!(bits(&r.m4(n0)), bits(&prop.node), "{at}: node matrix");
        assert_eq!(bits(&r.m4(n0 + 0x40)), bits(&prop.node_unscaled), "{at}: unscaled node matrix");
        assert_eq!(bits(&[std::array::from_fn(|k| r.f(n0 + 0xc0 + 4 * k as u32))]), bits(&[prop.center]), "{at}: sphere centre");
        assert_eq!(r.bytes(data + 0x48, 1)[0] != 0, *collides, "{at}: collision flag");
        if *collides {
            assert_eq!(bits(&r.m4(inst + 0x10)), bits(&prop.to_model), "{at}: world → model matrix");
            colliding.push((ids.len(), prop.center, prop.radius));
        }
        ids.push(node);
        node = r.u(node + 0x10);
    }
    assert_eq!(node, 0, "RAM list longer than {} props", created.len());

    let g = world::grid(&colliding);
    let minmax: Vec<u32> = (0..3).map(|k| r.u(GRID_MINMAX + 4 * k)).chain((0..3).map(|k| r.u(GRID_MINMAX + 0x10 + 4 * k))).collect();
    assert_eq!(minmax, g.min.iter().chain(&g.max).map(|v| v.to_bits()).collect::<Vec<_>>(), "grid box");
    let b: Vec<i32> = (0..6).map(|k| r.i(GRID_BOUNDS + 8 * k)).collect();
    assert_eq!(b, [g.lo[0], g.hi[0], g.lo[1], g.hi[1], g.lo[2], g.hi[2]], "grid cell bounds");
    assert_eq!([r.i(GRID_DIMS), r.i(GRID_DIMS + 8), r.i(GRID_DIMS + 16)], g.dims(), "grid dims");
    let mut listed = 0;
    for (c, want) in g.cells.iter().enumerate() {
        let cell = r.u(GRID) + 8 * c as u32;
        let got: Vec<usize> = (0..r.i(cell).max(0) as u32).map(|k| ids.iter().position(|&n| n == r.u(r.u(cell + 4) + 4 * k)).unwrap()).collect();
        assert_eq!(&got, want, "cell {c}");
        listed += got.len();
    }
    eprintln!("{} props, {} collide, {} cells, {listed} entries", ids.len(), colliding.len(), g.cells.len());
}
