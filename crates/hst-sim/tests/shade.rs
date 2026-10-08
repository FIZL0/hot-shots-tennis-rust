//! The ball's sun-shade light scale against the game, frame by frame: `context/p17t/ball05.txt` (slot 5, court 10, a
//! bot rally; per frame the ball's position x y z w and its model's light scale, as bits; made by
//! `research/p17t_ball_rec.py`) with the game's own map (`context/p17t/map05.bin`). On the court the height is from
//! y 0, off it from a ray cast down at court 10's collision model built from the disc; a miss keeps the last
//! scale. Skips when the recording or the disc is absent.
//! Also the sun-shade map built from the disc's court 10 against the game's (`context/p17l/map05.bin`, slot 5).

use hst_data::{iso::Iso, layout, mdl, mtl, xb::Archive};
use hst_sim::court;
use hst_sim::shade::{self, Caster, Frame, Shape};

#[test]
fn ball_light_scale_matches_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(rec), Ok(map), Ok(mut iso)) = (
        std::fs::read_to_string(format!("{root}/context/p17t/ball05.txt")),
        std::fs::read(format!("{root}/context/p17t/map05.bin")),
        Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")),
    ) else {
        eprintln!("recording or disc missing, skipped");
        return;
    };
    let world = court::world(&mut iso, 10);
    let frame = shade::Frame::new([-113.8475, -10.730181, -141.48601], [102.94255, 17.31305, 158.12808], 1.0, [0.0; 3]);
    let (mut exact, mut off, mut missed, mut bad) = (0, 0, 0, Vec::new());
    let mut last = None;
    for line in rec.lines().filter(|l| !l.starts_with('#')) {
        let w: Vec<u32> = line.split_whitespace().skip(1).map(|h| u32::from_str_radix(h, 16).unwrap()).collect();
        let pos = [0, 1, 2, 3].map(|k| f32::from_bits(w[k]));
        let want = w[4];
        if pos[0] == 0.0 && pos[2] == 0.0 {
            // the between-points reset puts the ball at the origin after that frame's light update
            last = Some(want);
            continue;
        }
        if pos[0].abs() > 10.685 || pos[2].abs() > 19.885 {
            off += 1;
        }
        let got = match shade::ball_height(&world.court, &world.models, pos) {
            Some(h) => Some(shade::ball(&map, &frame, pos[0], pos[2], h).to_bits()),
            None => {
                missed += 1;
                last
            }
        };
        match got {
            Some(g) if g == want => exact += 1,
            None => {}
            Some(g) => bad.push(format!("{line}: port {g:08x}")),
        }
        last = Some(want);
    }
    eprintln!("{exact} frames bit-exact, {off} off the court, {missed} ray misses");
    assert!(bad.is_empty(), "{} frames differ:\n{}", bad.len(), bad[..bad.len().min(20)].join("\n"));
    assert!(off > 20 && exact > 1250);
    // far outside the court model the ray hits nothing
    assert_eq!(shade::ball_height(&world.court, &world.models, [500.0, -0.5, 500.0, 1.0]), None);
}

#[test]
fn court10_map_near_game() {
    let (Ok(mut iso), Ok(game)) = (
        Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")),
        std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/p17l/map05.bin")),
    ) else {
        return;
    };
    let (mut models, mut shapes) = (std::collections::HashMap::new(), std::collections::HashMap::new());
    let mut read = |xb: &str, suffix: &str| {
        let d = iso.read(&format!("COURT/10/{xb}")).unwrap();
        let arc = Archive::parse(&d).unwrap();
        for e in arc.entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".mdl")) {
            let stem = &e.name[..e.name.len() - 4];
            if let Ok(m) = mdl::parse(&arc.read(e).unwrap()) {
                let mtl = arc.find(&format!("{stem}.MTL")).and_then(|x| arc.read(x).ok());
                let mti = arc.find(&format!("{stem}.MTI")).and_then(|x| arc.read(x).ok());
                let key = stem.rsplit('\\').next().unwrap().to_ascii_lowercase();
                if let Some(mats) = mtl.and_then(|t| mtl::parse(&t, mti.as_deref()).ok()) {
                    shapes.insert(key.clone(), Shape::new(&m, &mats));
                }
                models.insert(key, m);
            }
        }
        arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(suffix)).map(|e| arc.read(e).unwrap())
    };
    let list = layout::entries(&String::from_utf8_lossy(&read("CMN.XB", "entry_c10.txt").unwrap()));
    read("GRD01.XB", "-");
    let plants = layout::plants(&read("HOL01.XB", "plant_c10_h01_0.dat").unwrap()).unwrap();
    let tris = |m: &mdl::Model| -> Vec<[[f32; 3]; 3]> { m.materials.iter().flatten().flat_map(|pk| pk.triangles.iter().map(|t| t.map(|i| pk.vertices[i as usize].pos))).collect() };
    let hole = list.iter().find(|e| e.dir == "hole" && e.stem.contains("_h01") && layout::in_season(&e.stem, 0) && models.contains_key(&e.stem)).unwrap();
    let casters: Vec<Caster> = plants
        .iter()
        .filter(|p| p.code[3] != b'0' && (17..=19).contains(&p.category))
        .filter_map(|p| {
            let m = shapes.get(&layout::resolve(&list, p, 0)?.stem)?.clone();
            let s = if p.scale > 0.0 { p.scale } else { 1.0 };
            let (sn, c) = p.yaw.sin_cos();
            Some(Caster { axes: [[c * s, 0.0, -sn * s], [0.0, s, 0.0], [sn * s, 0.0, c * s]], pos: p.pos, tris: m.tris, cut: m.cut })
        })
        .collect();
    assert_eq!(casters.len(), 17);
    // the hole's frame (`frame_matches_game_ram`) and the sun as the game left them
    let f = Frame::new([-113.8475, -10.730181, -141.48601], [102.94255, 17.31305, 158.12808], 1.0, [0.0; 3]);
    let map = shade::build(&f, [-0.5877856, 0.7694209, 0.2499991], &casters, &tris(&models[&hole.stem]));
    let ones = |m: &[u8]| m.iter().map(|b| b.count_ones()).sum::<u32>();
    let both: Vec<u8> = map.iter().zip(&game).map(|(a, b)| a & b).collect();
    let either: Vec<u8> = map.iter().zip(&game).map(|(a, b)| a | b).collect();
    let iou = ones(&both) as f64 / ones(&either) as f64;
    // ponytail: fitted screen (`shade::build`); bit for bit P17v
    assert!(iou >= 0.975, "IoU {iou}");
}

/// The shade receiver vertices bit for bit against the game's (`context/p17v/rcv.txt`: every receiver triangle
/// vertex of the hole drawn in `context/p17v/cap0.pkl` with the camera, model and caster light matrices from game
/// RAM; slot 5, court 10). Skips when absent.
#[test]
fn receiver_vertices_match_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(txt) = std::fs::read_to_string(format!("{root}/context/p17v/rcv.txt")) else {
        eprintln!("recording missing, skipped");
        return;
    };
    let hex = |s: &str| u32::from_str_radix(s, 16).unwrap();
    let mat = |w: &[&str]| -> [[f32; 4]; 4] { std::array::from_fn(|i| std::array::from_fn(|j| f32::from_bits(hex(w[i * 4 + j])))) };
    let (mut vp, mut item, mut tex, mut n, mut bad) = ([[0.0; 4]; 4], [[0.0; 4]; 4], Vec::new(), 0, Vec::new());
    for line in txt.lines().filter(|l| !l.starts_with('#')) {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w[0] {
            "VP" => vp = mat(&w[1..]),
            "ITEM" => item = mat(&w[1..]),
            "L" => tex.push(shade::tex_matrix(&mat(&w[2..]))),
            c => {
                let p = [1, 2, 3].map(|k| f32::from_bits(hex(w[k])));
                let got = shade::receiver_vertex(&item, &vp, &tex[c.parse::<usize>().unwrap()], p);
                let want = ([hex(w[4]) as u16, hex(w[5]) as u16], [6, 7, 8].map(|k| f32::from_bits(hex(w[k]))));
                n += 1;
                if got != want {
                    bad.push(format!("{line}: port {got:?}"));
                }
            }
        }
    }
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(20)].join("\n"));
    assert!(n > 1700);
}

/// The casters' shadow textures bit for bit: `context/p17v/pass.txt` (`research/p17v2_fixture.py`: the load pass's
/// draws from `context/p17v/load10.gs` and the 16 static textures the game left in `context/p17v/cap0.gs`; slot 5,
/// court 10). Skips when absent.
#[test]
fn shadow_textures_match_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(txt) = std::fs::read_to_string(format!("{root}/context/p17v/pass.txt")) else {
        eprintln!("pass.txt missing, skipped");
        return;
    };
    let (mut draws, mut n, mut bad) = (Vec::<shade::PassDraw>::new(), 0, Vec::new());
    let wrap = |w: &str| match w {
        "r" => shade::Wrap::Repeat,
        c => {
            let (a, b) = c[1..].split_once(',').unwrap();
            shade::Wrap::Clamp(a.parse().unwrap(), b.parse().unwrap())
        }
    };
    for line in txt.lines().filter(|l| !l.starts_with('#')) {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w[0] {
            "T" => draws.clear(),
            "D" => draws.push(shade::PassDraw {
                tris: Vec::new(),
                aref: w[1].parse().unwrap(),
                modulate: w[2] == "1",
                tex: (w.len() > 3).then(|| shade::PassTexture {
                    log2: [w[3].parse().unwrap(), w[4].parse().unwrap()],
                    wrap: [wrap(w[5]), wrap(w[6])],
                    alpha: (0..w[7].len()).step_by(2).map(|i| u8::from_str_radix(&w[7][i..i + 2], 16).unwrap()).collect(),
                }),
            }),
            "V" => {
                let t = std::array::from_fn(|k| {
                    let f = &w[1 + 5 * k..];
                    let bits = |s: &str| f32::from_bits(u32::from_str_radix(s, 16).unwrap());
                    shade::PassVertex { xy: [f[0].parse().unwrap(), f[1].parse().unwrap()], st: [bits(f[2]), bits(f[3])], a: f[4].parse().unwrap() }
                });
                draws.last_mut().unwrap().tris.push(t);
            }
            _ => {
                let want: Vec<u8> = w[1].bytes().map(|c| (c as char).to_digit(16).unwrap() as u8).collect();
                let got = shade::pass_texture(&draws);
                let diff = got.iter().zip(&want).filter(|(a, b)| a != b).count();
                if diff > 0 {
                    bad.push(format!("texture {n}: {diff} texels differ"));
                }
                n += 1;
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
    assert_eq!(n, 16);
}
