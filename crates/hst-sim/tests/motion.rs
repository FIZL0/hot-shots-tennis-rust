//! The motion numbers of strokes, serves and post-point reactions through two doubles matches (slot 5's
//! `context/fixtures/match_s05.bin`, right-handed characters 0, 2, 1, 5; `1p3goodcpus.bin`, characters 6, 11, 2, 10,
//! the first two left-handed), from the recorded contact-search results and point outcomes.

use hst_sim::motion::{reaction, serve_walk, soft_follow, stroke_start, team_reactions, whiff, SWING_LEAD};
use hst_sim::replay::{Frame, frames_live};

fn p_u8(fr: Frame, p: usize, off: usize) -> u8 {
    fr.player_f32(p, off & !3).to_bits().to_le_bytes()[off & 3]
}
fn p_i32(fr: Frame, p: usize, off: usize) -> i32 {
    fr.player_f32(p, off).to_bits() as i32
}
fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
/// The match recordings and their save states' RAM.
const MATCHES: [(&str, &str); 2] = [("match_s05.bin", "slot5_ee.bin"), ("1p3goodcpus.bin", "1p3goodcpus_ee.bin")];

/// Every match present: its name, recording and per player the character (+0x12bc) and hand (+0x12b4) in its RAM.
fn matches() -> Vec<(&'static str, Vec<u8>, [(i32, f32); 4])> {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    MATCHES
        .iter()
        .filter_map(|&(name, ram)| {
            let (Ok(data), Ok(ram)) = (std::fs::read(format!("{dir}/{name}")), std::fs::read(format!("{dir}/{ram}"))) else {
                eprintln!("{name} or {ram} absent, skipped");
                return None;
            };
            let ru = |a: usize| u32::from_le_bytes(ram[a & 0x1ff_ffff..][..4].try_into().unwrap()) as usize;
            let lineup = [0, 1, 2, 3].map(|p| ru(ru(0x422f80) + 0xa8 + 4 * p)).map(|pl| (ru(pl + 0x12bc) as i32, f(&ram, pl + 0x12b4)));
            Some((name, data, lineup))
        })
        .collect()
}

#[test]
fn match_stroke_motions() {
    for (name, data, lineup) in matches() {
        let frames = frames_live(&data);
        let (mut starts, mut switches, mut whiffs, mut softs) = (0, 0, 0, 0);
        for k in 1..frames.len() - 1 {
            let (a, fr, next) = (frames[k - 1], frames[k], frames[k + 1]);
            if f(fr.gm(), 0x58).to_bits() as i32 - f(a.gm(), 0x58).to_bits() as i32 > 2 {
                break;
            }
            for p in 0..4 {
                if p_u8(fr, p, 0x3fa4) != 0 {
                    continue;
                }
                let (was, now) = (p_u8(a, p, 0x3fa5), p_u8(fr, p, 0x3fa5));
                let (m0, m1) = (p_i32(a, p, 0x3df0), p_i32(fr, p, 0x3df0));
                let (branch, left) = (p_u8(fr, p, 0x3ec1), p_i32(fr, p, 0x3ec4));
                if now == 2 && was != 2 {
                    // the swing is +0x3e40 while it waits, +0x3e44 once playing
                    let pending = p_i32(fr, p, 0x3e40);
                    let anim = if pending >= 0 { pending } else { p_i32(fr, p, 0x3e44) };
                    let (m, _, wait) = stroke_start(branch, left, anim);
                    assert_eq!((m, wait.is_some()), (m1, pending >= 0), "{name} start k={k} p={p}");
                    starts += 1;
                } else if now == 2 && was == 2 && m1 != m0 {
                    if left == SWING_LEAD && p_i32(a, p, 0x3e40) >= 0 {
                        assert_eq!(m1, p_i32(a, p, 0x3e40), "{name} switch k={k} p={p}");
                        switches += 1;
                    } else if (0x27..=0x2a).contains(&m1) {
                        assert_eq!(whiff(m0), Some(m1), "{name} whiff k={k} p={p}");
                        whiffs += 1;
                    }
                }
                // the contact: countdown 1 → −1 and this player the last hitter
                if p_i32(a, p, 0x3ec4) == 1 && left == -1 && fr.global(0x423058) == p as i32 {
                    let b = fr.live_ball();
                    let v = [f(b, 0x140), f(b, 0x144), f(b, 0x148)];
                    let soft = soft_follow(branch, p_i32(fr, p, 0x3e44), v, p_i32(fr, p, 0x3f50) as u32, lineup[p].1);
                    let m = p_i32(next, p, 0x3df0);
                    match soft {
                        Some(s) => assert_eq!(m, s, "{name} soft k={k} p={p}"),
                        None => assert!(!(0x1c..=0x1d).contains(&m), "{name} no soft k={k} p={p}"),
                    }
                    softs += 1;
                }
            }
        }
        eprintln!("{name}: {starts} starts, {switches} switches, {whiffs} whiffs, {softs} contacts");
        assert!(starts > 30 && switches > 5 && whiffs > 0 && softs > 30, "{name}");
    }
}

#[test]
fn match_serve_walk() {
    // only Carol serves in 1p3goodcpus.bin, and she doesn't walk
    let mut total = 0;
    for (name, data, lineup) in matches() {
        let frames = frames_live(&data);
        let mut n = 0;
        for k in 1..frames.len() {
            let (a, fr) = (frames[k - 1], frames[k]);
            for p in 0..4 {
                let m = p_i32(fr, p, 0x3df0);
                if p_u8(fr, p, 0x3fa4) != 1 || !(0x21..=0x22).contains(&m) {
                    continue;
                }
                let dx = fr.player_pos(p)[0] - a.player_pos(p)[0];
                if dx == 0.0 {
                    continue;
                }
                let fwd = if fr.player_pos(p)[2] < 0.0 { 1.0 } else { -1.0 };
                assert_eq!(serve_walk(dx, fwd, lineup[p].1), m, "{name} k={k} p={p}");
                n += 1;
            }
        }
        eprintln!("{name}: {n} serve walk frames");
        total += n;
    }
    assert!(total > 1000, "{total}");
}

/// Every post-point reaction: the outcome's base reaction or a team reaction of the character's set not taken
/// by a player updated before it (the draw itself is the game's random number).
#[test]
fn match_reactions() {
    for (name, data, lineup) in matches() {
        let frames = frames_live(&data);
        let (mut n, mut team) = (0, 0);
        let mut taken: Vec<i32> = vec![];
        for k in 1..frames.len() {
            let (a, fr) = (frames[k - 1], frames[k]);
            if f(fr.gm(), 0x58).to_bits() as i32 - f(a.gm(), 0x58).to_bits() as i32 > 2 {
                break;
            }
            for p in 0..4 {
                if p_u8(fr, p, 0x3fa4) != 2 || p_u8(a, p, 0x3fa4) == 2 {
                    continue;
                }
                if p == 0 || (0..p).all(|q| p_u8(a, q, 0x3fa4) == 2 || p_u8(fr, q, 0x3fa4) != 2) {
                    taken.clear();
                }
                let won = (p as i32 & 1) == fr.global(0x4230a8);
                let base = reaction(p_u8(a, p, 0x3fa5) == 3, 4, won, fr.global(0x4230b8) != 0, false);
                let got = p_i32(fr, p, 0x3db0);
                if got >= 0x30 {
                    let c = got - 0x30;
                    assert!((0x2c..=0x2d).contains(&base) && team_reactions(lineup[p].0).contains(&c) && !taken.contains(&c), "{name} k={k} p={p} {got:#x}");
                    taken.push(c);
                    team += 1;
                } else {
                    assert_eq!(got, base, "{name} k={k} p={p}");
                }
                n += 1;
            }
        }
        eprintln!("{name}: {n} reactions ({team} team)");
        assert!(n > 10 && team > 5, "{name}");
    }
}

/// The ANI sampler (squad rotations, Hermite positions, bone-length scale) against the players' skeletons in
/// five save-state RAM dumps (`context/ram/s0N.bin`): every multi-key track's quaternion, rotation rows and
/// position at the time last sampled, bit for bit. Character and costume are found by the motion's keys and
/// the skeleton's rest bones.
#[test]
fn clip_sampler_ram() {
    use hst_data::iso::Iso;
    use hst_sim::pose::{Clip, q_matrix};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let Ok(mut iso) = Iso::open(format!("{root}Hot Shots Tennis (USA).iso")) else { return eprintln!("ISO absent, skipped") };
    let (mut tracks, mut players) = (0, 0);
    for s in ["s03", "s04", "s05", "s08", "s09"] {
        let Ok(ram) = std::fs::read(format!("{root}context/ram/{s}.bin")) else { return eprintln!("{s}.bin absent, skipped") };
        let u = |a: usize| u32::from_le_bytes(ram[(a & 0x1ff_ffff)..][..4].try_into().unwrap()) as usize;
        let fr = |a: usize| f32::from_bits(u(a) as u32);
        let cstr = |a: usize| { let a = a & 0x1ff_ffff; String::from_utf8_lossy(&ram[a..a + ram[a..].iter().position(|&b| b == 0).unwrap()]).into_owned() };
        let gm = u(0x422f80);
        for p in 0..4 {
            let pl = u(gm + 0xa8 + 4 * p);
            let an = u(pl + 0x54);
            // +0x3c is already the next frame's time; +0x38 is the time last sampled
            let (motion, t, clip) = (u(an + 0x20), fr(an + 0x38), u(an + 0x24));
            let track = |k: usize| u(u(clip + 0x10) + 4 * k);
            let list_n = |l: usize| if l == 0 { 0 } else { u(u(l + 0xc)) };
            let nodes = u(u(u(u(an)) + 0xc) + 0x64);
            let node_of = |k: usize| nodes + ((u(u(clip + 0x1c) + 2 * (k & !1)) >> (16 * (k & 1))) as u16 as usize) * 0x120;
            let (a, sk) = ram_clip(&mut iso, &ram, an, clip, motion);
            let ours = Clip::new(&sk, &a);
            assert_eq!(ours.tracks.len(), u(clip + 0xc), "{s} p{p} track count");
            assert_eq!(ours.length.to_bits(), fr(clip + 0x2c).to_bits(), "{s} p{p} length");
            for k in 0..ours.tracks.len() {
                let name = cstr(u(track(k) + 8));
                assert_eq!(sk.names[ours.tracks[k].node], name, "{s} p{p} track {k}");
                assert_eq!(ours.tracks[k].scale.to_bits(), fr(u(clip + 0x14) + 4 * k).to_bits(), "{s} p{p} {name} scale");
                let node = node_of(k);
                let cache = u(clip + 0x18) + 0x40 * k;
                let (rot, pos) = ours.sample(k, t);
                let ctx = format!("{s} p{p} motion {motion:#x} t {t} {name}");
                if list_n(u(track(k) + 0xc)) > 1 {
                    let q = rot.unwrap();
                    assert_eq!(q.map(f32::to_bits), [0, 4, 8, 12].map(|o| u(node + 0xe0 + o) as u32), "{ctx} quat");
                    let mm = q_matrix(q);
                    for r in 0..3 {
                        assert_eq!(mm[r][..3].iter().map(|v| v.to_bits()).collect::<Vec<_>>(), (0..3).map(|j| u(cache + 16 * r + 4 * j) as u32).collect::<Vec<_>>(), "{ctx} row {r}");
                    }
                }
                if list_n(u(track(k) + 0x10)) > 1 {
                    assert_eq!(pos.unwrap().map(f32::to_bits), [0, 4, 8].map(|o| u(node + 0xf0 + o) as u32), "{ctx} position");
                }
                tracks += 1;
            }
            players += 1;
        }
    }
    eprintln!("{players} players, {tracks} tracks bit-exact");
}

/// Post-point reactions carry the player along their `*_dummy` path (team reactions co03–co05 forward only,
/// ×0.55 for character 5; `gu_set` the whole path of the character's own dummy): the accumulated spot +0x3da0
/// of every reacting frame of each match, bit-exact, with the motion time = frames since the reaction began
/// (held at the motion's length), counted in game frames (gm+0x50): a sample can catch two or none (spot unchanged).
#[test]
fn match_reaction_root() {
    use hst_data::{ani, iso::Iso, xb::Archive};
    use hst_sim::motion::reaction_root;
    use hst_sim::pose::Path;
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else { return eprintln!("ISO absent, skipped") };
    let cg = iso.read("PCDATA/PCCG0.XB").unwrap();
    let carc = Archive::parse(&cg).unwrap();
    let file = |stem: String| {
        carc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(&format!("mtgrl/{stem}.ani2"))).map(|e| ani::parse(&carc.read(e).unwrap()).unwrap())
    };
    let co = ["co01_f", "co02_f", "co03", "co04", "co05"];
    let paths: Vec<Option<Path>> = co.iter().map(|n| file(format!("re_pc00_{n}_dummy")).and_then(|a| Path::new(&a))).collect();
    let lens: Vec<f32> = co.iter().map(|n| { let a = file(format!("re_pc00_{n}")).unwrap(); a.end_tick() as f32 / a.ticks_per_frame as f32 }).collect();
    // gu_set (0x2e) and its root path (motion 0x35) per character
    let mut gu_set = std::collections::HashMap::new();
    let matches = matches();
    for c in matches.iter().flat_map(|m| m.2.map(|p| p.0)) {
        let arc_data = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
        let arc = Archive::parse(&arc_data).unwrap();
        let get = |m: usize| { let stem = ani::motion_name(m, c as usize).unwrap().to_ascii_lowercase(); arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))).map(|e| ani::parse(&arc.read(e).unwrap()).unwrap()) };
        let a = get(0x2e).unwrap();
        gu_set.insert(c, (get(0x35).and_then(|d| Path::new(&d)), a.end_tick() as f32 / a.ticks_per_frame as f32));
    }
    for (name, data, lineup) in &matches {
        let frames = frames_live(&data);
        let (mut exact, mut held, mut gu, mut t) = (0, 0, 0, [0f32; 4]);
        for k in 1..frames.len() {
            let (a, fr) = (frames[k - 1], frames[k]);
            // game frames (gm+0x50) since the last sample: now and then a sample catches two and the next none
            let ticks = f(fr.gm(), 0x50).to_bits().wrapping_sub(f(a.gm(), 0x50).to_bits());
            if ticks == 0 {
                continue;
            }
            for p in 0..4 {
                if p_u8(fr, p, 0x3fa4) != 2 || p_u8(fr, p, 0x3fa7) != 1 {
                    continue;
                }
                if p_u8(a, p, 0x3fa7) != 1 {
                    t[p] = 0.0;
                } else {
                    t[p] += (ticks - 1) as f32;
                }
                let m = p_i32(fr, p, 0x3db0);
                let v = |f: Frame, o: usize| [0, 4, 8, 12].map(|d| f.player_f32(p, o + d));
                let (pt, team) = if m >= 0x30 {
                    let k = (m - 0x30) as usize;
                    (paths[k].as_ref().map_or([0.0; 4], |c| c.at(t[p].min(lens[k]))), true)
                } else if m == 0x2e {
                    let (path, len) = &gu_set[&lineup[p].0];
                    let Some(path) = path else { continue };
                    gu += 1;
                    (path.at(t[p].min(*len)), false)
                } else {
                    continue;
                };
                t[p] += 1.0;
                let got = reaction_root(pt, team, lineup[p].0, [v(fr, 0x3d40), v(fr, 0x3d50), v(fr, 0x3d60)], v(fr, 0x3d90), v(a, 0x3da0));
                let want = v(fr, 0x3da0);
                if got.map(f32::to_bits) == want.map(f32::to_bits) {
                    exact += 1;
                } else {
                    assert_eq!(want, v(a, 0x3da0), "{name} k={k} p={p} motion {m:#x} t={}: {got:?}", t[p] - 1.0);
                    held += 1;
                }
            }
        }
        eprintln!("{name}: {exact} reacting frames bit-exact ({gu} gu_set), {held} held at the phase's end");
        assert!(exact > 500 && held < 50, "{name}");
    }
}

/// Each player's motion clock through 9000 frames of the slot-5 match (`context/fixtures/anim_s05.bin`,
/// `tools/record_anim.py`): sampled time (+0x38) and time (+0x3c) bit-exact after every tick, held through the soft
/// follow-through's crossfade. The motion player's own countdown (+0x60, one step per tick whether held or not)
/// counts the ticks: the game skips a phase change's frame and runs two the next.
#[test]
fn anim_s05_clock() {
    use hst_sim::motion::Clock;
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/anim_s05.bin")) else { return eprintln!("anim_s05.bin absent, skipped") };
    const S: usize = 4 + 0x100 + 4 * 0x484;
    // (motion, clip, sampled, time, speed, loop, countdown, held, length)
    let read = |k: usize, p: usize| {
        let b = &data[k * S + 0x104 + p * 0x484..];
        let (an, i) = (&b[0x400..], |o: usize| i32::from_le_bytes(b[0x400 + o..0x404 + o].try_into().unwrap()));
        (i(0x20), i(0x24), f(an, 0x38), f(an, 0x3c), f(an, 0x40), an[0x78] != 0, i(0x60), an[0x6c] != 0, f(b, 0x480))
    };
    let (mut ticks, mut sets, mut held) = (0, 0, 0);
    for k in 1..data.len() / S {
        for p in 0..4 {
            let (a, b) = (read(k - 1, p), read(k, p));
            let n = a.6 - b.6;
            let mut c = Clock { time: a.3, sampled: a.2, speed: a.4, looping: a.5, hold: a.7.then_some(a.6) };
            if a.0 == b.0 && a.1 == b.1 && (0..=2).contains(&n) {
                for _ in 0..n {
                    c.tick(a.8);
                }
                ticks += n;
                held += a.7 as i32;
            } else {
                // set this frame: restarted at 0 (a crossfade's countdown from 7), then up to two ticks
                let start = Clock::start(b.4, b.5, b.7.then_some(7));
                let fits = (0..=2).find(|&j| {
                    c = start;
                    (0..j).for_each(|_| c.tick(b.8));
                    (c.sampled, c.time, c.hold.is_some()) == (b.2, b.3, b.7) && (!b.7 || c.hold == Some(b.6))
                });
                assert!(fits.is_some(), "set k={k} p={p} {a:?} -> {b:?}");
                sets += 1;
                continue;
            }
            assert_eq!((c.sampled.to_bits(), c.time.to_bits(), c.hold.is_some()), (b.2.to_bits(), b.3.to_bits(), b.7), "k={k} p={p} {a:?} -> {b:?}");
        }
    }
    eprintln!("{ticks} ticks ({held} held), {sets} sets");
    assert!(ticks > 30000 && sets > 100 && held > 5);
}

/// The arm table of the player struct at `pl` in a RAM image: the costume whose arm nodes have the player's rest
/// translations, the character's stroke motions 0x10–0x19.
fn ram_arm_table(iso: &mut hst_data::iso::Iso, ram: &[u8], pl: usize) -> (hst_sim::pose::ArmTable, i32) {
    use hst_data::{ani, mdl, xb::Archive};
    use hst_sim::pose::{arm_table, Clip, Skeleton};
    let u = |a: usize| u32::from_le_bytes(ram[(a & 0x1ff_ffff)..][..4].try_into().unwrap()) as usize;
    let c = u(pl + 0x12bc) as i32;
    let rest_of = |k: usize| [0, 4, 8].map(|o| u(u(u(u(pl + 0x17f0 + 4 * k) + 0x108) + 0x10) + 0x70 + o) as u32);
    let want: Vec<[u32; 3]> = [0, 9, 11, 12, 13].map(rest_of).to_vec();
    let names = ["Racket", "Bip01RHand", "Bip01RForearm", "Bip01RUpperArm", "Bip01LUpperArm"];
    let sk = (0..10)
        .filter_map(|costume| {
            let mdata = iso.read(&format!("PC/PC{c:02}C{costume:02}.XB")).ok()?;
            let marc = Archive::parse(&mdata).unwrap();
            let e = marc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(".mdl"))?;
            let m = mdl::parse(&marc.read(e).unwrap()).unwrap();
            Some(Skeleton { names: m.node_names.clone(), parent: m.node_parent.clone(), rest: m.node_local.clone() })
        })
        .find(|sk| names.iter().zip(&want).all(|(n, w)| sk.names.iter().position(|x| x == n).map(|i| [0, 1, 2].map(|j| sk.rest[i][3][j].to_bits())) == Some(*w)))
        .unwrap_or_else(|| panic!("no costume of character {c} matches"));
    let data = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
    let arc = Archive::parse(&data).unwrap();
    let clips: Vec<Clip> = (0x10..0x1c)
        .map(|m| {
            let stem = ani::motion_name(m, c as usize).unwrap().to_ascii_lowercase();
            let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))).unwrap();
            Clip::new(&sk, &ani::parse(&arc.read(e).unwrap()).unwrap())
        })
        .collect();
    (arm_table(&sk, &clips, c), c)
}

/// The arm table each player builds at load (stroke motions 0x10–0x19 at frame 8: arm locals, the upper arms'
/// parent chains, the aimed upper arm, its yaw, the shoulder, reach and radius per stroke, and the three
/// averages), bit-exact against the player structs of every RAM image.
#[test]
fn arm_table_ram() {
    use hst_data::iso::Iso;
    use hst_sim::pose::M4;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let Ok(mut iso) = Iso::open(format!("{root}Hot Shots Tennis (USA).iso")) else { return eprintln!("ISO absent, skipped") };
    let mut players = 0;
    for s in ["s03", "s04", "s05", "s08", "s09"] {
        let Ok(ram) = std::fs::read(format!("{root}context/ram/{s}.bin")) else { return eprintln!("{s}.bin absent, skipped") };
        let u = |a: usize| u32::from_le_bytes(ram[(a & 0x1ff_ffff)..][..4].try_into().unwrap()) as usize;
        let gm = u(0x422f80);
        for p in 0..4 {
            let pl = u(gm + 0xa8 + 4 * p);
            let (t, c) = ram_arm_table(&mut iso, &ram, pl);
            let ctx = format!("{s} p{p} char {c}");
            let bits = |a: usize, n: usize| (0..n).map(|k| u(a + 4 * k) as u32).collect::<Vec<_>>();
            let mbits = |m: &M4| m.iter().flatten().map(|v| v.to_bits()).collect::<Vec<_>>();
            for (i, a) in t.poses.iter().enumerate().skip(10) {
                let b = pl + 0x140 * (i - 10);
                for (name, m, off) in [("racket", &a.racket, 0x3220), ("r_hand", &a.r_hand, 0x3260), ("r_forearm", &a.r_forearm, 0x32a0), ("r_upper", &a.r_upper, 0x32e0), ("r_chain", &a.r_chain, 0x3320)] {
                    assert_eq!(mbits(m), bits(b + off, 16), "{ctx} volley {i} {name}");
                }
                assert_eq!(a.shoulder.map(f32::to_bits).to_vec(), bits(pl + 0x34a0 + 0x10 * (i - 10), 4), "{ctx} volley {i} shoulder");
            }
            for (i, a) in t.poses.iter().enumerate().take(10) {
                let b = pl + 0x200 * i;
                for (name, m, off) in [("racket", &a.racket, 0x18e0), ("r_hand", &a.r_hand, 0x1920), ("r_forearm", &a.r_forearm, 0x1960), ("r_upper", &a.r_upper, 0x19a0), ("l_upper", &a.l_upper, 0x19e0), ("r_chain", &a.r_chain, 0x1a20), ("l_chain", &a.l_chain, 0x1a60), ("aimed", &a.r_upper_aimed, 0x1aa0)] {
                    assert_eq!(mbits(m), bits(b + off, 16), "{ctx} stroke {i} {name}");
                }
                assert_eq!(mbits(&a.yaw), bits(pl + 0x2ce0 + 0x40 * i, 16), "{ctx} stroke {i} yaw");
                assert_eq!(a.shoulder.map(f32::to_bits).to_vec(), bits(pl + 0x2f60 + 0x10 * i, 4), "{ctx} stroke {i} shoulder");
                assert_eq!(a.reach.to_bits(), u(pl + 0x3000 + 4 * i) as u32, "{ctx} stroke {i} reach");
                assert_eq!(a.radius.to_bits(), u(pl + 0x3028 + 4 * i) as u32, "{ctx} stroke {i} radius");
            }
            assert_eq!([t.reach, t.side, t.hand_x].map(f32::to_bits).to_vec(), bits(pl + 0x3050, 3), "{ctx} averages");
            players += 1;
        }
    }
    eprintln!("{players} players' arm tables bit-exact");
}

/// The contact solve at every stroke of the slot-5 match (`context/fixtures/anim_s05.bin`, player +0x3c00..+0x4000
/// per frame): on the frame the solve flag +0x3fc8 rises, the body step +0x3fd0 from the contact point +0x3fb0 and
/// the position +0x3fe0, for the stroke whose solve reproduces it; the frames +0x3fc0/+0x3fc4 from +0x3ec4 (the
/// second already advanced once that frame). Arm tables from `context/ram/s05.bin` (same match).
#[test]
fn contact_solve_anim() {
    use hst_data::iso::Iso;
    use hst_sim::pose::{contact_solve, ik_frames};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let Ok(mut iso) = Iso::open(format!("{root}Hot Shots Tennis (USA).iso")) else { return eprintln!("ISO absent, skipped") };
    let (Ok(ram), Ok(anim)) = (std::fs::read(format!("{root}context/ram/s05.bin")), std::fs::read(format!("{root}context/fixtures/anim_s05.bin"))) else {
        return eprintln!("s05 captures absent, skipped");
    };
    let u = |a: usize| u32::from_le_bytes(ram[(a & 0x1ff_ffff)..][..4].try_into().unwrap()) as usize;
    let gm = u(0x422f80);
    let pls: Vec<usize> = (0..4).map(|p| u(gm + 0xa8 + 4 * p)).collect();
    let tables: Vec<_> = pls.iter().map(|&pl| ram_arm_table(&mut iso, &ram, pl).0).collect();
    const S: usize = 4 + 0x100 + 4 * 0x484;
    let blk = |k: usize, p: usize| &anim[k * S + 0x104 + p * 0x484..][..0x400];
    let v4 = |b: &[u8], o: usize| [0, 4, 8, 12].map(|j| f(b, o - 0x3c00 + j));
    let i32_at = |b: &[u8], o: usize| f(b, o - 0x3c00).to_bits() as i32;
    let (mut tried, mut other) = (0, 0);
    for k in 1..anim.len() / S - 20 {
        for p in 0..4 {
            let (b, prev) = (blk(k, p), blk(k - 1, p));
            if !(b[0x3c8] == 1 && prev[0x3c8] == 0) {
                continue;
            }
            let n = i32_at(b, 0x3ec4);
            let (f0, f1) = ik_frames(n);
            assert_eq!((f0, f1 + 1), (i32_at(b, 0x3fc0), i32_at(b, 0x3fc4)), "frame {k} p{p} frames of {n}");
            // the stroke the search chose: the first stroke-range motion the player starts within 20 frames
            let m = (k..k + 20).map(|j| i32_at(&anim[j * S + 0x104 + p * 0x484..][..0x484], 0x3c00 + 0x420)).find(|&m| m >= 0x10).unwrap();
            if !(0x10..0x1c).contains(&m) {
                other += 1; // higher motions index past the table (serves step 0, 0x1f under 1 cm): no IK
                continue;
            }
            tried += 1;
            let want = v4(b, 0x3fd0).map(f32::to_bits);
            // the side scale (+0x12b0, ±1) flips at the change of ends: +1 on the near (−z) side
            let scale = [if v4(b, 0x3fe0)[2] < 0.0 { 1.0 } else { -1.0 }, 1.0];
            let r = contact_solve(&tables[p], m as usize - 0x10, v4(b, 0x3fb0), v4(b, 0x3fe0), scale);
            assert_eq!(r.step.map(f32::to_bits), want, "frame {k} p{p} motion {m:#x}: {:?}", r.step);
        }
    }
    eprintln!("{tried} contact solves bit-exact ({other} outside strokes and volleys 0x10-0x1b)");
    assert!(tried > 35);
}

/// The contact IK's frames after each recorded solve of the slot-5 match: the counter +0x3fc4, the flag +0x3fc8
/// and the body stepped to the solve position plus step·weight (+0x3d70), bit-exact (mover taken as unclamped);
/// the motion speed back at 1 from the frame before contact.
#[test]
fn arm_ik_anim() {
    use hst_sim::pose::{ArmIk, ArmSolve};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let Ok(anim) = std::fs::read(format!("{root}context/fixtures/anim_s05.bin")) else { return eprintln!("anim_s05 absent, skipped") };
    const S: usize = 4 + 0x100 + 4 * 0x484;
    let blk = |k: usize, p: usize| &anim[k * S + 0x104 + p * 0x484..][..0x400];
    let v4 = |b: &[u8], o: usize| [0, 4, 8, 12].map(|j| f(b, o - 0x3c00 + j));
    let i32_at = |b: &[u8], o: usize| f(b, o - 0x3c00).to_bits() as i32;
    let (mut runs, mut frames, mut units) = (0, 0, 0);
    for k in 1..anim.len() / S {
        for p in 0..4 {
            let b = blk(k, p);
            if !(b[0x3c8] == 1 && blk(k - 1, p)[0x3c8] == 0) {
                continue;
            }
            let solve = ArmSolve { step: v4(b, 0x3fd0), quats: [[0.0; 4]; 4] };
            let mut ik = ArmIk { solve, frames: i32_at(b, 0x3fc0), n: i32_at(b, 0x3fc4), on: true, free: true, pos0: v4(b, 0x3fe0), seen: v4(b, 0x3d70) };
            for j in k + 1..anim.len() / S {
                let b = blk(j, p);
                let mut pos = v4(blk(j - 1, p), 0x3d70);
                let (_, unit) = ik.tick(&mut pos, |p, d| (std::array::from_fn(|i| hst_sim::ps2::add(p[i], d[i])), true));
                let ctx = format!("solve {k} p{p} frame {j}");
                // the motion speed (anim +0x34) set to 1 the frame before contact
                let speed = |b: &[u8]| f(&b[0x400..], 0x34);
                let anim_of = |j: usize| &anim[j * S + 0x104 + p * 0x484..][..0x484];
                if unit {
                    assert_eq!(speed(anim_of(j)), 1.0, "{ctx} speed");
                    units += (speed(anim_of(j - 1)) != 1.0) as i32;
                }
                assert_eq!((ik.on, ik.n), (b[0x3c8] == 1, i32_at(b, 0x3fc4)), "{ctx}");
                assert_eq!([pos[0], pos[2]].map(f32::to_bits), [0, 8].map(|o| f(b, 0x170 + o).to_bits()), "{ctx}");
                frames += 1;
                if !ik.on {
                    break;
                }
            }
            runs += 1;
        }
    }
    eprintln!("{runs} contact IK runs, {frames} frames bit-exact, {units} speed resets");
    assert!(runs > 40);
}

/// The motion player's crossfade through 9000 frames of the slot-5 match (`context/fixtures/anim_s05.bin`, motion
/// object +0x44..+0x6c): outgoing motion, its sampled time and clock, length, countdown, weight and hold flag
/// bit-exact after every frame, from the setter's length rule (8 for looping motions and whiffs 0x27/0x28, the
/// recorded count for the held soft follow-through, else a cut) and the per-frame steps. The outgoing clip's
/// length is the one recorded while it played.
#[test]
fn anim_s05_fade() {
    use hst_sim::motion::{Clock, Fade};
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/anim_s05.bin")) else { return eprintln!("anim_s05.bin absent, skipped") };
    const S: usize = 4 + 0x100 + 4 * 0x484;
    // (motion, clip, clock, fade, old clip, length)
    let read = |k: usize, p: usize| {
        let b = &data[k * S + 0x104 + p * 0x484..];
        let (an, i) = (&b[0x400..], |o: usize| i32::from_le_bytes(b[0x400 + o..0x404 + o].try_into().unwrap()));
        let clock = Clock { time: f(an, 0x3c), sampled: f(an, 0x38), speed: f(an, 0x40), looping: an[0x78] != 0, hold: None };
        let fade = Fade {
            id: i(0x44),
            clip: i(0x48) != 0,
            sampled: f(an, 0x4c),
            time: f(an, 0x50),
            speed: f(an, 0x54),
            looping: an[0x58] != 0,
            frames: i(0x5c),
            count: i(0x60),
            weight: f(an, 0x64),
            hold: an[0x6c] != 0,
        };
        (i(0x20), i(0x24), clock, fade, i(0x48), f(b, 0x480))
    };
    let bits = |f: &Fade| (f.id, f.clip, f.sampled.to_bits(), f.time.to_bits(), f.speed.to_bits(), f.looping, f.frames, f.count, f.weight.to_bits(), f.hold);
    let mut lengths = std::collections::HashMap::new();
    let (mut frames, mut sets, mut fading, mut miss, mut held, mut switched) = (0, 0, 0, Vec::new(), 0, 0);
    for k in 1..data.len() / S {
        for p in 0..4 {
            let (a, b) = (read(k - 1, p), read(k, p));
            lengths.insert(a.1, a.5);
            let run = |mut f: Fade, n: usize, clip: i32| {
                let len = lengths.get(&clip).copied().unwrap_or(0.0);
                for _ in 0..n {
                    f.tick(len);
                    f.tick_hold();
                }
                f
            };
            let want = bits(&b.3);
            if (0..=2).any(|n| bits(&run(a.3, n, a.4)) == want) && a.0 == b.0 && a.1 == b.1 {
                frames += 1;
                fading += a.3.clip as i32;
                continue;
            }
            // the setter, then up to two frames
            let (id, looping, hold) = (b.0, b.2.looping, b.3.hold);
            let n = if hold { b.3.frames } else if looping || id == 0x27 || id == 0x28 { 8 } else { 0 };
            let mut f = a.3;
            f.start(n, id, hold, a.0, a.1 != 0, &a.2);
            // at a point's end the player is reset first (no motion, no clip, a cut), then set
            let mut r = a.3;
            r.start(0, -1, false, -1, false, &a.2);
            r.start(n, id, hold, -1, false, &a.2);
            if (0..=2).any(|j| bits(&run(f, j, b.4)) == want || bits(&run(r, j, b.4)) == want) {
                sets += 1;
                held += hold as i32;
                switched += (a.3.clip && n >= 2) as i32;
            } else {
                miss.push(format!("k={k} p={p} {:?} {:?} -> {:?}", (a.0, a.2), a.3, (b.0, b.3)));
            }
        }
    }
    eprintln!("{frames} frames ({fading} fading), {sets} sets ({held} held, {switched} mid-fade), {} missed", miss.len());
    for m in miss.iter().take(10) {
        eprintln!("{m}");
    }
    assert!(miss.is_empty() && sets > 100 && fading > 500 && held > 5 && switched > 5);
}

/// The character, motion file and skeleton of the clip at `clip` (motion `motion`) of player anim object `an`:
/// the character whose motion file has the clip's first rotation keys, the costume whose rest bones match.
fn ram_clip(iso: &mut hst_data::iso::Iso, ram: &[u8], an: usize, clip: usize, motion: usize) -> (hst_data::ani::Anim, hst_sim::pose::Skeleton) {
    use hst_data::{ani, mdl, xb::Archive};
    use hst_sim::pose::Skeleton;
    let u = |a: usize| u32::from_le_bytes(ram[(a & 0x1ff_ffff)..][..4].try_into().unwrap()) as usize;
    let cstr = |a: usize| { let a = a & 0x1ff_ffff; String::from_utf8_lossy(&ram[a..a + ram[a..].iter().position(|&b| b == 0).unwrap()]).into_owned() };
    let track = |k: usize| u(u(clip + 0x10) + 4 * k);
    let list_n = |l: usize| if l == 0 { 0 } else { u(u(l + 0xc)) };
    let first_rot = |k: usize| { let l = u(track(k) + 0xc); (0..list_n(l)).map(|j| [0, 4, 8, 12].map(|o| u(u(l + 0x18) + 16 * j + o) as u32)).collect::<Vec<_>>() };
    let want0 = first_rot(0);
    let nodes = u(u(u(u(an)) + 0xc) + 0x64);
    let node_of = |k: usize| nodes + ((u(u(clip + 0x1c) + 2 * (k & !1)) >> (16 * (k & 1))) as u16 as usize) * 0x120;
    let names: Vec<String> = (0..u(clip + 0xc)).map(|k| cstr(u(track(k) + 8))).collect();
    let rests: Vec<[u32; 3]> = (0..names.len()).map(|k| [0, 4, 8].map(|o| u(u(u(node_of(k) + 0x108) + 0x10) + 0x70 + o) as u32)).collect();
    for c in 0..16 {
        let Ok(data) = iso.read(&format!("PCANI/PC{c:02}ANI.XB")) else { continue };
        let arc = Archive::parse(&data).unwrap();
        let stem = ani::motion_name(motion, c).unwrap().to_ascii_lowercase();
        let Some(e) = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))) else { continue };
        let a = ani::parse(&arc.read(e).unwrap()).unwrap();
        let keys: Vec<[u32; 4]> = a.tracks.iter().find(|t| t.name == names[0]).map(|t| t.rotation.iter().map(|k| k.1.map(f32::to_bits)).collect()).unwrap_or_default();
        if keys != want0 {
            continue;
        }
        for costume in 0..10 {
            let Ok(mdata) = iso.read(&format!("PC/PC{c:02}C{costume:02}.XB")) else { continue };
            let marc = Archive::parse(&mdata).unwrap();
            let Some(e) = marc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(".mdl")) else { continue };
            let m = mdl::parse(&marc.read(e).unwrap()).unwrap();
            let sk = Skeleton { names: m.node_names.clone(), parent: m.node_parent.clone(), rest: m.node_local.clone() };
            let rest = |n: &String| sk.names.iter().position(|x| x == n).map(|i| [0, 1, 2].map(|j| sk.rest[i][3][j].to_bits()));
            if names.iter().zip(&rests).all(|(n, r)| rest(n) == Some(*r)) {
                return (a, sk);
            }
        }
    }
    panic!("no character's motion {motion:#x} matches the clip at {clip:#x}");
}

/// The crossfade's mix in the save states caught mid-fade: the first 23 tracks of the outgoing clip (the player's
/// blend count) hold, in its cache, the rotation rows of slerp(new pose, outgoing pose at its sampled time, w) and
/// the position new + (outgoing − new)·w, with w the weight before this frame's step, bit-exact.
#[test]
fn fade_mix_ram() {
    use hst_data::iso::Iso;
    use hst_sim::{motion::mix, pose::{Clip, q_matrix}};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let Ok(mut iso) = Iso::open(format!("{root}Hot Shots Tennis (USA).iso")) else { return eprintln!("ISO absent, skipped") };
    let (mut tracks, mut players) = (0, 0);
    for s in ["s03", "s04", "s05", "s08", "s09"] {
        let Ok(ram) = std::fs::read(format!("{root}context/ram/{s}.bin")) else { return eprintln!("{s}.bin absent, skipped") };
        let u = |a: usize| u32::from_le_bytes(ram[(a & 0x1ff_ffff)..][..4].try_into().unwrap()) as usize;
        let fr = |a: usize| f32::from_bits(u(a) as u32);
        let gm = u(0x422f80);
        for p in 0..4 {
            let an = u(u(gm + 0xa8 + 4 * p) + 0x54);
            let (old, clip, count) = (u(an + 0x44), u(an + 0x48), u(an + 0x60) as i32);
            if clip == 0 || ram[(an + 0x6c) & 0x1ff_ffff] != 0 {
                continue;
            }
            assert_eq!((u(an + 0x70), u(an + 0x74) as i32), (23, -1), "{s} p{p} blend count, special track");
            let (a, sk) = ram_clip(&mut iso, &ram, an, clip, old);
            let ours = Clip::new(&sk, &a);
            let (t, w) = (fr(an + 0x4c), hst_sim::ps2::div((count + 1) as f32, u(an + 0x5c) as f32));
            let nodes = u(u(u(u(an)) + 0xc) + 0x64);
            for k in 0..23.min(ours.tracks.len()) {
                let node = nodes + ((u(u(clip + 0x1c) + 2 * (k & !1)) >> (16 * (k & 1))) as u16 as usize) * 0x120;
                let cache = u(clip + 0x18) + 0x40 * k;
                let cur = ([0, 4, 8, 12].map(|o| fr(node + 0xe0 + o)), [0, 4, 8].map(|o| fr(node + 0xf0 + o)));
                let (rot, pos) = ours.sample(k, t);
                let (q, x) = mix(cur, (rot.unwrap_or(cur.0), pos.unwrap_or(cur.1)), w);
                let ctx = format!("{s} p{p} motion {old:#x} t {t} w {w} track {k}");
                if rot.is_some() {
                    let m = q_matrix(q);
                    for r in 0..3 {
                        assert_eq!(m[r][..3].iter().map(|v| v.to_bits()).collect::<Vec<_>>(), (0..3).map(|j| u(cache + 16 * r + 4 * j) as u32).collect::<Vec<_>>(), "{ctx} row {r}");
                    }
                }
                if pos.is_some() {
                    assert_eq!(x.map(f32::to_bits), [0, 4, 8].map(|o| u(cache + 0x30 + o) as u32), "{ctx} position");
                }
                tracks += 1;
            }
            players += 1;
        }
    }
    eprintln!("{players} players mid-fade, {tracks} tracks bit-exact");
    assert!(players >= 3);
}

/// Every stroke's hand-off in the slot-5 match (`context/fixtures/anim_s05.bin`; player +0x3f00 frames since contact,
/// +0x3e50 recovery, +0x3ec1 branch, +0x3ee4 shot code): played out the frame after the motion's sampled time
/// reached its length, broken off into standing or running no earlier than `follow_over` allows with input, and the
/// recovery from the shot.
#[test]
fn anim_s05_follow_through() {
    use hst_sim::motion::{follow_over, recovery};
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(data) = std::fs::read(format!("{dir}/anim_s05.bin")) else { return eprintln!("anim_s05.bin absent, skipped") };
    const S: usize = 4 + 0x100 + 4 * 0x484;
    let blk = |k: usize, p: usize| &data[k * S + 0x104 + p * 0x484..][..0x484];
    let i = |b: &[u8], o: usize| i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let pl = |b: &[u8], o: usize| i(b, o - 0x3c00);
    let (mut ends, mut cuts, mut earliest) = (0, 0, 0);
    for p in 0..4 {
        for k in 1..data.len() / S {
            let (a, b) = (blk(k - 1, p), blk(k, p));
            let (ma, mb) = (i(a, 0x420), i(b, 0x420));
            if ma == mb || !(0x10..=0x1d).contains(&ma) || mb >= 0x10 || pl(b, 0x3f00) == 0 {
                continue;
            }
            let (after, rec) = (pl(b, 0x3f00) as u32, pl(b, 0x3e50) as u32);
            let played = |blk: &[u8]| f(blk, 0x480) <= f(blk, 0x438);
            let kind = match pl(b, 0x3ee4) { 0x10 => 4, 4 => 3, 8 => 2, 2 => 1, _ => 0 };
            assert_eq!(recovery(b[0x3ec1 - 0x3c00], kind), rec, "k={k} p={p}");
            if played(a) {
                assert!(follow_over(after, rec, true, false) && !played(blk(k - 2, p)), "end k={k} p={p} {ma:#x} after {after}");
                ends += 1;
            } else {
                assert!(follow_over(after, rec, false, true), "cut k={k} p={p} {ma:#x} after {after} rec {rec}");
                earliest += !follow_over(after - 1, rec, false, true) as i32;
                cuts += 1;
            }
        }
    }
    eprintln!("{ends} played out, {cuts} broken off ({earliest} on the first frame allowed)");
    assert!(ends > 10 && cuts > 10);
}

/// Every recorded whiff, frame by frame through `motion::Whiff`: the swing's countdown (−3 with a miss motion to
/// come, −2 without) until the pose, then the miss motion, the frames since the pose and the re-press lock; a
/// new press only where `press()` takes one (quiet as recorded), and the player freed only past the recovery.
#[test]
fn recorded_whiffs() {
    use hst_sim::motion::{WHIFF_RECOVERY, Whiff};
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let (mut whiffs, mut repress, mut freed) = (0, 0, 0);
    for name in ["new_recording.bin", "round1.bin", "match_s05.bin", "lob_smash_s05.bin", "1p3goodcpus.bin"] {
        let Ok(data) = std::fs::read(format!("{dir}/{name}")) else {
            eprintln!("{name} absent, skipped");
            continue;
        };
        let frames = if data.len() % hst_sim::replay::SAMPLE_LIVE == 0 { frames_live(&data) } else { hst_sim::replay::frames(&data) };
        let (cd, state, sub) = (|fr, p| p_i32(fr, p, 0x3ec4), |fr, p| p_u8(fr, p, 0x3fa4), |fr, p| p_u8(fr, p, 0x3fa5));
        let started = |fr, p, before| {
            let c = cd(fr, p);
            sub(fr, p) == 2 && (c == -3 || c == -2) && c != before && matches!(p_i32(fr, p, 0x3df0), 0x10..=0x19 | 0x1f)
        };
        for p in 0..4 {
            let mut k = 1;
            while k < frames.len() {
                let fr = frames[k];
                if !started(fr, p, cd(frames[k - 1], p)) {
                    k += 1;
                    continue;
                }
                let mut w = Whiff::new(p_i32(fr, p, 0x3df0), cd(fr, p) == -3, p_u8(fr, p, 0x3f04) != 0);
                whiffs += 1;
                let k0 = k;
                let at = |j: usize| format!("{name} p{p} press {k0} frame {j}");
                k += 1;
                while k < frames.len() {
                    let fr = frames[k];
                    if state(fr, p) != state(frames[k - 1], p) || fr.gm()[0x58..0x5c] == frames[k - 1].gm()[0x58..0x5c] {
                        break; // the point's reactions or the next serve, or the players' logic stopped (a fault)
                    }
                    let miss = w.step();
                    if sub(fr, p) != 2 {
                        assert!(w.since().is_some_and(|s| s >= WHIFF_RECOVERY), "freed early: {}", at(k));
                        freed += 1;
                        break;
                    }
                    if cd(fr, p) != -1 && w.since().is_some() {
                        // a new press: the next whiff starts here, or a contact
                        assert_eq!(w.press(), Some(p_u8(fr, p, 0x3f04) != 0), "re-press: {}", at(k));
                        repress += 1;
                        break;
                    }
                    let Some(s) = w.since() else {
                        assert_eq!(cd(fr, p), if w.miss { -3 } else { -2 }, "before the pose: {}", at(k));
                        k += 1;
                        continue;
                    };
                    assert_eq!(cd(fr, p), -1, "{}", at(k));
                    assert_eq!(p_i32(fr, p, 0x3f00), s as i32, "since the pose: {}", at(k));
                    assert_eq!(p_i32(fr, p, 0x3e54), w.lock().map_or(-1, |l| l as i32), "re-press lock: {}", at(k));
                    if s == 0 {
                        assert_eq!(miss, w.miss.then(|| whiff(w.anim)).flatten(), "{}", at(k));
                        if let Some(m) = miss {
                            assert_eq!(p_i32(fr, p, 0x3df0), m, "miss motion: {}", at(k));
                        }
                    }
                    k += 1;
                }
            }
        }
    }
    eprintln!("{whiffs} whiffs, {repress} re-presses, {freed} freed");
}
