//! The sun-shade map built from the disc's court 10 against the game's (`context/p17l/map05.bin`, slot 5).
use hst_data::{iso::Iso, layout, mdl, xb::Archive};
use hst_sim::shade::{self, Caster, Frame};

#[test]
fn court10_map_near_game() {
    let (Ok(mut iso), Ok(game)) = (
        Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")),
        std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/p17l/map05.bin")),
    ) else {
        return;
    };
    let mut models = std::collections::HashMap::new();
    let mut read = |xb: &str, suffix: &str| {
        let d = iso.read(&format!("COURT/10/{xb}")).unwrap();
        let arc = Archive::parse(&d).unwrap();
        for e in arc.entries.iter().filter(|e| e.name.to_ascii_lowercase().ends_with(".mdl")) {
            if let Ok(m) = mdl::parse(&arc.read(e).unwrap()) {
                models.insert(e.name[..e.name.len() - 4].rsplit('\\').next().unwrap().to_ascii_lowercase(), m);
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
            let m = models.get(&layout::resolve(&list, p, 0)?.stem)?;
            let s = if p.scale > 0.0 { p.scale } else { 1.0 };
            let (sn, c) = p.yaw.sin_cos();
            Some(Caster { axes: [[c * s, 0.0, -sn * s], [0.0, s, 0.0], [sn * s, 0.0, c * s]], pos: p.pos, tris: tris(m) })
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
    // ponytail: solid casters and fitted screen (`shade::build`); leaf alpha P17u, bit for bit P17v
    assert!(iou > 0.94, "IoU {iou}");
}
