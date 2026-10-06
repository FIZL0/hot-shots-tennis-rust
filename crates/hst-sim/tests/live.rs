//! Replays the live ball of a recorded bot match (`context/live/net_s05.bin`, not in git; made by
//! `tools/record_live.py 5 …` from save-state slot 5, court 10) one frame at a time: each frame's ball object is
//! loaded into a `Flight`, stepped once against court 10's collision world (built from the disc: the court model
//! and the grid of props) and compared bit for bit with the next recorded frame: airborne frames and every contact
//! (court, ground around it, walls, net, net cord, the ghost material) must match exactly. Skips when the recording
//! or the disc is absent.

use hst_data::iso::Iso;
use hst_data::xb::Archive;
use hst_data::{exe, layout, mdl, mtl};
use hst_sim::ball::{Ball, COURTS, Flight, Material, Params, Shot};
use hst_sim::mesh::{self, World};
use hst_sim::world::{self, IDENTITY};

const SAMPLE: usize = 4 + 0x290 + 0x290 + 0x40;
const COURT: usize = 10;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn v3(b: &[u8], o: usize) -> [f32; 3] {
    [f(b, o), f(b, o + 4), f(b, o + 8)]
}
fn rows(b: &[u8], o: usize) -> [[f32; 4]; 4] {
    std::array::from_fn(|r| std::array::from_fn(|k| f(b, o + 16 * r + 4 * k)))
}

/// The ball object (0x290 bytes from the live ball pointer `*(gm+0x88)`) as a flight and its shot.
fn load(b: &[u8]) -> (Flight, Shot) {
    let ball = Ball { pos: v3(b, 0xe0), vel: v3(b, 0x130), spin: f(b, 0x1a4) };
    let mut fl = Flight::new(ball, rows(b, 0x160), rows(b, 0x1c0));
    fl.frame = i(b, 0xac);
    fl.bounces = i(b, 0x224);
    fl.contacts = i(b, 0x228);
    fl.special_contacts = i(b, 0x22c);
    fl.rolling = b[0xa4] == 2;
    fl.slide = f(b, 0x200);
    let shot = Shot {
        params: Params::default(),
        curve: f(b, 0x254),
        bend: f(b, 0x250),
        side: v3(b, 0x90),
        curve_frames: i(b, 0x260),
        wind: v3(b, 0x240),
        first_bounce_spin: f(b, 0x1a8),
        first_bounce_restitution: f(b, 0x1ac),
        class: b[0x58],
        kind: i(b, 0x5c),
    };
    (fl, shot)
}

/// A disc model as collision geometry.
fn collision_model(m: &mdl::Model, mt: &mtl::Mtl) -> mesh::Model {
    let u16_at = |h: &[u8], o: usize| u16::from_le_bytes([h[o], h[o + 1]]) as i32;
    mesh::Model {
        tris: m
            .colliding(mt)
            .map(|t| mesh::Tri { node: t.node, material: t.material, pos: t.pos, uv: t.uv, bounds: t.bounds })
            .collect(),
        materials: mt
            .materials
            .iter()
            .zip(&m.wrap)
            .map(|(mat, &wrap)| mesh::Material {
                two_sided: mat.two_sided,
                textured: mat.texture.is_some(),
                wrap,
                map: mat.attributes.map(|a| {
                    let a = &mt.attributes[a];
                    mesh::AttributeMap {
                        width: u16_at(&a.header, 2),
                        height: u16_at(&a.header, 4),
                        stride: u16_at(&a.header, 0x14),
                        texels: a.texels.clone(),
                        table: a.table,
                    }
                }),
            })
            .collect(),
        node_from_model: m.node_inverse.clone(),
    }
}

/// Court 10, hole 1: the court model at the origin and the colliding props in the game's list order and grid.
fn court_world(iso: &mut Iso) -> World {
    let mut files = std::collections::HashMap::new();
    let (mut list, mut plants, mut court) = (Vec::new(), Vec::new(), None);
    for arc in ["CMN.XB", "GRD01.XB", "HOL01.XB"] {
        let data = iso.read(&format!("COURT/{COURT}/{arc}")).unwrap();
        let a = Archive::parse(&data).unwrap();
        let read = |n: &str| a.entries.iter().find(|e| e.name.eq_ignore_ascii_case(n)).map(|e| a.read(e).unwrap());
        for e in &a.entries {
            let name = e.name.to_ascii_lowercase();
            if name.ends_with(&format!("entry_c{COURT}.txt")) {
                list = layout::entries(&String::from_utf8_lossy(&a.read(e).unwrap()));
            } else if name.ends_with(&format!("plant_c{COURT}_h01_0.dat")) {
                plants = layout::plants(&a.read(e).unwrap()).unwrap();
            } else if let Some(stem) = name.strip_suffix(".mdl") {
                let base = &e.name[..e.name.len() - 4];
                let m = mdl::parse(&a.read(e).unwrap()).unwrap();
                let mt = mtl::parse(&read(&format!("{base}.MTL")).unwrap(), read(&format!("{base}.MTI")).as_deref()).unwrap();
                if arc == "GRD01.XB" && m.collides(&mt) {
                    assert!(court.is_none(), "one collision model in the court archive");
                    court = Some((m, mt));
                } else {
                    files.insert(stem.rsplit(['/', '\\']).next().unwrap().to_string(), (m, mt));
                }
            }
        }
    }
    let (cm, cmt) = court.expect("court collision model");
    // the court stands at the origin, unscaled, every node in place
    for (k, n) in cm.node_local.iter().chain(&cm.node_inverse).enumerate() {
        assert_eq!(*n, IDENTITY, "court node matrix {k}");
    }
    let mut models = vec![collision_model(&cm, &cmt)];
    let court = mesh::Object {
        model: 0,
        scale: 1.0,
        to_model: IDENTITY,
        nodes: vec![IDENTITY; cm.node_count],
        center: cm.center,
        radius: cm.radius,
    };

    let mut created = Vec::new();
    for cat in [15, 17, 18, 19, 20] {
        for p in plants.iter().filter(|p| p.category == cat && p.scale != 0.0) {
            let Some(e) = layout::resolve(&list, p, 0) else { continue };
            let (m, _) = &files[&e.stem];
            let (at, _) = world::place(p.category, p.pos, p.yaw, p.code);
            created.push((e.stem.clone(), world::instance(at, p.scale, &m.node_local[0], m.center, m.radius), p.scale));
        }
    }
    let (mut props, mut stems, mut spheres) = (Vec::new(), Vec::<String>::new(), Vec::new());
    for k in world::list_order(created.len()) {
        let (stem, prop, scale) = &created[k];
        let (m, mt) = &files[stem];
        if !m.collides(mt) {
            continue;
        }
        assert_eq!(m.node_count, 1, "{stem}: props are single-node");
        let model = match stems.iter().position(|s| s == stem) {
            Some(i) => i + 1,
            None => {
                stems.push(stem.clone());
                models.push(collision_model(m, mt));
                models.len() - 1
            }
        };
        spheres.push((props.len(), prop.center, prop.radius));
        props.push(mesh::Object { model, scale: *scale, to_model: prop.to_model, nodes: vec![prop.node], center: prop.center, radius: m.radius });
    }
    World { models, court, props, grid: world::grid(&spheres) }
}

#[test]
fn live_ball_frames_match_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(data), Ok(mut iso), Ok(cnf), Ok(bin)) = (
        std::fs::read(format!("{root}/context/live/net_s05.bin")),
        Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")),
        std::fs::read(format!("{root}/context/iso/SYSTEM.CNF")),
        std::fs::read(format!("{root}/context/iso/ZZBIN/GAME.BIN")),
    ) else {
        eprintln!("recording or disc missing, skipped");
        return;
    };
    let world = court_world(&mut iso);
    let materials: Vec<Material> = exe::Game::new(&cnf, &bin)
        .unwrap()
        .surfaces()
        .iter()
        .map(|s| Material { court: s.court, special: s.special, restitution: s.restitution, spin_loss: s.spin_loss })
        .collect();

    let s: Vec<&[u8]> = data.chunks_exact(SAMPLE).collect();
    let (mut exact, mut touches, mut failures) = (0, std::collections::BTreeMap::new(), Vec::new());
    for w in s.windows(2) {
        let (a, b) = (&w[0][4..4 + 0x290], &w[1][4..4 + 0x290]);
        let vsync = i(w[1], 0);
        // only frames the ball physically flies: consecutive vsyncs, same shot, in play (+0xa4 0 or 1)
        // and not carried (the server's toss/bounce moves it with zero velocity)
        let carried = |o: &[u8]| v3(o, 0x130) == [0.0; 3];
        if vsync != i(w[0], 0) + 1 || i(b, 0xac) != i(a, 0xac) + 1 || a[0xa4] > 1 || b[0xa4] > 1 || carried(a) || carried(b) {
            continue;
        }
        let (mut fl, shot) = load(a);
        fl.step_world(&shot, &COURTS[COURT], &world, &materials);
        let want = [v3(b, 0xe0), v3(b, 0x130)].concat();
        let got = [fl.ball.pos, fl.ball.vel].concat();
        let same = (0..6).all(|k| got[k].to_bits() == want[k].to_bits());
        // a contact moves a counter or changes the material of the last contact
        if i(b, 0x224) != i(a, 0x224) || i(b, 0x228) != i(a, 0x228) || i(b, 0x22c) != i(a, 0x22c) || b[0x220] != a[0x220] {
            *touches.entry(b[0x220]).or_insert(0) += 1;
        }
        if same {
            exact += 1;
        } else {
            failures.push(format!("vsync {vsync} material {} pos {:?}: got {got:?} want {want:?}", b[0x220], v3(b, 0xe0)));
        }
    }
    eprintln!("{exact} frames bit-exact; contact frames by material: {touches:?}");
    assert!(failures.is_empty(), "{} frames diverged:\n{}", failures.len(), failures[..failures.len().min(20)].join("\n"));
    assert!(exact > 12000);
}
