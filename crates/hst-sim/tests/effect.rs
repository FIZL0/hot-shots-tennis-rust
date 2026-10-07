//! The racket-impact effects against a recorded bot match (`context/fixtures/effects_s05.bin`,
//! `tools/record_effects.py 5`): from the frame the game starts an impact, every frame it lives the port's clocks,
//! morph weights and material alphas match the game's bit for bit, it ends the same frame, and its scale follows
//! from the ball's velocity.

use hst_data::{ani, iso::Iso, mdl, mor, mtl, xb::Archive};
use hst_sim::effect::{Effect, impact_matrix, impact_scale};

const KINDS: [&str; 6] = ["top", "slice", "flat", "lob", "drop", "smash"];

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

#[test]
fn impacts_s05() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(data), Ok(mut iso)) = (std::fs::read(format!("{dir}/effects_s05.bin")), Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso"))) else {
        return eprintln!("effects_s05.bin or ISO absent, skipped");
    };
    let arc_data = iso.read("AZUMA/C_EFF/EFFCT.XB0").unwrap();
    let arc = Archive::parse(&arc_data).unwrap();
    let get = |name: &str| arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(&name.to_ascii_lowercase())).map(|e| arc.read(e).unwrap());

    // the capture: header {6, per model MOR/MTA entry counts}; per frame vsync, effects 0x100, impact 0x60, ball 0x290,
    // marker 0x40, per model {model 0x80, mesh 0x60, MOR player 0x30, MTA player 0x30, entries 0x28/0x20, colours 0x10}
    let counts: Vec<usize> = (0..12).map(|i| u(&data, 4 + 4 * i) as usize).collect();
    let mut at = vec![];
    let mut size = 4 + 0x100 + 0x60 + 0x290 + 0x40;
    for k in 0..6 {
        at.push(size);
        size += 0x80 + 0x60 + 0x60 + counts[2 * k] * 0x28 + counts[2 * k + 1] * 0x30;
    }
    let frames: Vec<&[u8]> = data[0x34..].chunks_exact(size).collect();

    let mut fx: Vec<(Effect, Vec<usize>, Vec<usize>)> = KINDS
        .iter()
        .map(|k| {
            let s = format!("impact_{k}_a");
            let m = mdl::parse(&get(&format!("{s}.MDL")).unwrap()).unwrap();
            let a = ani::parse(&get(&format!("{s}.ANI")).unwrap()).unwrap();
            let mo = mor::parse(&get(&format!("{s}.MOR")).unwrap(), 1).unwrap();
            let mt = mor::parse(&get(&format!("{s}.MTA")).unwrap(), 1).unwrap();
            let mats = mtl::parse(&get(&format!("{s}.MTL")).unwrap(), get(&format!("{s}.MTI")).as_deref()).unwrap().materials;
            // the game's entries are the bound tracks in file order
            let wt = mo.tracks.iter().filter_map(|t| m.morph_names.iter().position(|n| *n == t.name)).collect();
            let at = mt.tracks.iter().filter_map(|t| mats.iter().position(|n| n.name == t.name)).collect();
            (Effect::new(&m, &a, &mo, &mt, &mats), wt, at)
        })
        .collect();
    for (k, (_, w, a)) in fx.iter().enumerate() {
        assert_eq!((w.len(), a.len()), (counts[2 * k], counts[2 * k + 1]), "{} binding", KINDS[k]);
    }

    let (mut live, mut seen) = (None::<usize>, [0; 6]);
    for (i, s) in frames.iter().enumerate() {
        let (e, imp) = (&s[4..0x104], &s[0x104..0x164]);
        let model = u(imp, 0x54);
        let k = (0..6).find(|&k| u(e, 0x84 + 4 * k) == model);
        let state = u(imp, 0x50);
        match (live, state, k) {
            (None, 1, Some(k)) => {
                fx[k].0.start();
                live = Some(k);
                seen[k] += 1;
                let ball = &s[0x164..];
                let want = f(&s[at[k]..], 0x80 + 0x40);
                let kind = u(e, 0xd0) as i32;
                // the model is the button's shot kind (the smash's for a smash), whatever the stroke: a framed
                // mis-hit pops up as a kind 3 volley but keeps its button's impact
                let (class, ball_kind) = (u(ball, 0x58), u(ball, 0x5c) as i32);
                assert_eq!(k, if class == 3 { 5 } else { kind as usize }, "frame {i} model");
                assert!(kind == ball_kind || (class == 2 && ball_kind == 3), "frame {i} kind {kind} ball kind {ball_kind}");
                let m = impact_matrix([f(ball, 0x140), f(ball, 0x144), f(ball, 0x148), f(ball, 0x14c)], [f(ball, 0xe0), f(ball, 0xe4), f(ball, 0xe8)]);
                let want_m: Vec<u32> = (0..16).map(|j| u(&s[at[k]..], 0x80 + 4 * j)).collect();
                assert_eq!(m.as_flattened().iter().map(|v| v.to_bits()).collect::<Vec<_>>(), want_m, "frame {i} matrix {m:?}");
                assert_eq!(impact_scale(kind, [f(ball, 0x140), f(ball, 0x144), f(ball, 0x148)]).to_bits(), want.to_bits(), "frame {i} scale");
            }
            (Some(k), _, _) => fx[k].0.tick(),
            _ => {}
        }
        let Some(k) = live else { continue };
        let (eff, wt, al) = &fx[k];
        let p = &s[at[k]..];
        let (mor, mta) = (&p[0xe0..0x110], &p[0x110..0x140]);
        assert_eq!(eff.live, state == 1, "frame {i} {} live", KINDS[k]);
        let t = eff.times();
        assert_eq!([t[0].to_bits(), t[1].to_bits(), t[2].to_bits()], [f(p, 0x38).to_bits(), f(mor, 0x1c).to_bits(), f(mta, 0x1c).to_bits()], "frame {i} {} times", KINDS[k]);
        for (j, &tg) in wt.iter().enumerate() {
            assert_eq!(eff.weights[tg].to_bits(), f(p, 0x140 + 0x28 * j + 0x24).to_bits(), "frame {i} {} weight {j}", KINDS[k]);
        }
        let te = 0x140 + 0x28 * wt.len();
        for (j, &tg) in al.iter().enumerate() {
            assert_eq!(eff.alphas[tg].to_bits(), f(p, te + 0x20 * j + 0xc).to_bits(), "frame {i} {} alpha {j}", KINDS[k]);
        }
        if !eff.live {
            live = None;
        }
    }
    eprintln!("impacts per kind {seen:?}");
    assert!(seen.iter().filter(|&&n| n > 0).count() >= 5, "kinds seen {seen:?}");
}

/// Every effect model on the disc with key channels builds and plays out to its end. (Not every track binds:
/// `impact_lob_a.MTA` drives a `Material #25` the model lacks, and the game skips it as the port does.)
#[test]
fn every_effect_binds() {
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else { return eprintln!("no ISO, skipped") };
    let arc_data = iso.read("AZUMA/C_EFF/EFFCT.XB0").unwrap();
    let arc = Archive::parse(&arc_data).unwrap();
    let get = |name: &str| arc.entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).map(|e| arc.read(e).unwrap());
    let mut n = 0;
    for e in arc.entries.iter().filter(|e| e.name.to_ascii_uppercase().ends_with(".MDL")) {
        let s = &e.name[..e.name.len() - 4];
        let (Some(a), Some(mo)) = (get(&format!("{s}.ANI")), get(&format!("{s}.MOR"))) else { continue };
        let m = mdl::parse(&arc.read(e).unwrap()).unwrap();
        let mo = mor::parse(&mo, 1).unwrap();
        let mt = get(&format!("{s}.MTA")).map_or(mor::Tracks { ticks_per_frame: 80, tracks: vec![] }, |d| mor::parse(&d, 1).unwrap());
        let mats = mtl::parse(&get(&format!("{s}.MTL")).unwrap(), get(&format!("{s}.MTI")).as_deref()).unwrap().materials;
        let mut fx = Effect::new(&m, &ani::parse(&a).unwrap(), &mo, &mt, &mats);
        fx.start();
        let frames = (0..1000).take_while(|_| {
            fx.tick();
            fx.live
        });
        assert!(frames.count() < 999, "{s}: never ends");
        assert_eq!(fx.locals().len(), m.node_names.len());
        n += 1;
    }
    eprintln!("{n} effects");
    assert!(n >= 6);
}

/// The yellow smash marker of a △ lob onto P1 (`context/fixtures/smash_mark_s04.txt`, N4a: slot 4 with the
/// opponents' strokes forced to lobs; line 1: P1's smash top and middle, the point the game placed; then the
/// game's predicted path, f32 bits). Fed 15 + 14 entries the launch frame and 15 a frame after, as the game grows
/// it, the search places the game's point on the frame the game did (the 5th: path of 75).
#[test]
fn smash_mark_s04() {
    use hst_sim::effect::{PathEntry, SmashSearch};
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(text) = std::fs::read_to_string(format!("{dir}/smash_mark_s04.txt")) else {
        return eprintln!("smash_mark_s04.txt absent, skipped");
    };
    let fl = |s: &str| f32::from_bits(u32::from_str_radix(s, 16).unwrap());
    let mut lines = text.lines().map(|l| l.split(' ').collect::<Vec<_>>());
    let head: Vec<f32> = lines.next().unwrap().iter().map(|s| fl(s)).collect();
    let path: Vec<PathEntry> = lines.map(|w| PathEntry { pos: [fl(w[0]), fl(w[1]), fl(w[2])], vel: [fl(w[3]), fl(w[4]), fl(w[5])], bounces: w[6].parse().unwrap() }).collect();
    let mut s = SmashSearch::new(&[[head[0], head[1]]]);
    let placed: Vec<usize> = (0..5).filter(|k| s.search(&path[..15 * (k + 1)])).collect();
    assert_eq!(placed, [4]);
    assert_eq!(s.at().map(|p| p.map(f32::to_bits)), Some([head[2].to_bits(), head[3].to_bits()]));
}

/// The yellow smash marker (`taguchi/other/smash_p`) doesn't end: the game clamps its clocks at the last frame and
/// draws it until the live ball bounces. Its morph track is 105 frames but the blob's alpha fades out over 120, so
/// a plain one-shot would cut the blob off at about half alpha.
#[test]
fn smash_mark_holds() {
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else { return eprintln!("no ISO, skipped") };
    let data = iso.read("PCDATA/PCCG0.XB").unwrap();
    let arc = Archive::parse(&data).unwrap();
    let get = |ext: &str| arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(&format!("taguchi/other/smash_p.{ext}"))).map(|e| arc.read(e).unwrap());
    let m = mdl::parse(&get("mdl").unwrap()).unwrap();
    let mats = mtl::parse(&get("mtl").unwrap(), get("mti").as_deref()).unwrap().materials;
    let mut fx = Effect::new(&m, &ani::Anim { ticks_per_frame: 1, tracks: vec![] }, &mor::parse(&get("mor").unwrap(), 1).unwrap(), &mor::parse(&get("mta").unwrap(), 1).unwrap(), &mats);
    fx.hold = true;
    fx.start();
    let alpha = |fx: &Effect| fx.alphas.iter().cloned().fold(0.0, f32::max);
    for _ in 0..110 {
        fx.tick();
    }
    assert!(fx.live && alpha(&fx) > 0.2, "fading at 110: {:?}", fx.alphas);
    for _ in 0..200 {
        fx.tick();
    }
    assert!(fx.live && alpha(&fx) == 0.0, "held, faded out: {:?} {:?}", fx.times(), fx.alphas);
}

/// The yellow smash marker in the doubles match with one human (Carol, P0) and three CPUs on court 11
/// (`1p3goodcpus.bin` with `tools/play_p2m2.py`'s `.mark` and `.path`): for each placement the game made, the
/// search fed the game's predicted path a frame (15 entries) at a time with Carol's smash heights places the point
/// on the same frame and at the same spot, and the marker holds until the live ball's next bounce or hit.
#[test]
fn smash_mark_goodcpus() {
    use hst_sim::effect::{PathEntry, SmashSearch};
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (Ok(live), Ok(paths), Ok(marks), Ok(ram)) = (
        std::fs::read(format!("{dir}/1p3goodcpus.bin")),
        std::fs::read(format!("{dir}/1p3goodcpus.bin.path")),
        std::fs::read(format!("{dir}/1p3goodcpus.bin.mark")),
        std::fs::read(format!("{dir}/1p3goodcpus_ee.bin")),
    ) else {
        return eprintln!("1p3goodcpus .bin/.path/.mark/_ee absent, skipped");
    };
    let frames = hst_sim::replay::frames_live(&live);
    // Carol's (character 6) TParam record in RAM: smash top and window middle
    let rec = 0x2f0880 + 6 * 0x118;
    let heights = [f(&ram, rec + 0xe8), f(&ram, rec + 0xec)];
    // .mark: u32 vsync + the marker object's bytes +0x50..+0x1b0 (count +0x144, point x/z +0x150/+0x158)
    let mark = |v: u32| marks.chunks(4 + 0x160).find(|c| u(c, 0) == v).map(|c| &c[4..]).unwrap();
    let (mut at, mut placed) = (0, 0);
    while at < paths.len() {
        let (v, len) = (u(&paths, at), u(&paths, at + 4) as usize);
        let path: Vec<PathEntry> = paths[at + 8..at + 8 + 0x30 * len]
            .chunks(0x30)
            .map(|e| PathEntry { pos: [f(e, 0), f(e, 4), f(e, 8)], vel: [f(e, 0x10), f(e, 0x14), f(e, 0x18)], bounces: u(e, 0x20) as i32 })
            .collect();
        at += 8 + 0x30 * len;
        let mut s = SmashSearch::new(&[heights]);
        let first: Vec<usize> = (0..len / 15).filter(|k| s.search(&path[..15 * (k + 1)])).collect();
        assert_eq!(first, [len / 15 - 1], "vsync {v}: path of {len}");
        let m = mark(v);
        assert_eq!(s.at().map(|p| p.map(f32::to_bits)), Some([f(m, 0x100).to_bits(), f(m, 0x108).to_bits()]), "vsync {v}");
        // held until the live ball's next bounce (+0x224) or hit (its flight frame +0xac restarts): the count
        // (+0x144) is cleared that frame
        let k = frames.iter().position(|fr| fr.vsync() == v).unwrap();
        let ball = |j: usize, o: usize| u(frames[j].live_ball(), o);
        let bounce = (k + 1..frames.len()).find(|&j| ball(j, 0x224) > ball(k, 0x224) || ball(j, 0xac) < ball(j - 1, 0xac)).map(|j| frames[j].vsync());
        let cleared = marks.chunks(4 + 0x160).map(|c| (u(c, 0), u(c, 4 + 0xf4))).find(|&(w, n)| w > v && n == 0).map(|c| c.0);
        assert_eq!(cleared, bounce, "vsync {v}: marker cleared, ball bounced or hit");
        placed += 1;
    }
    eprintln!("{placed} smash points placed");
    assert!(placed >= 1);
}
