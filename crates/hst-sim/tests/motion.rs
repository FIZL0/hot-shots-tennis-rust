//! The motion numbers of strokes, serves and post-point reactions through the slot-5 doubles match
//! (`context/fixtures/match_s05.bin`), from the recorded contact-search results and point outcomes.

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
const CHARS: [i32; 4] = [0, 2, 1, 5];

fn load() -> Option<Vec<u8>> {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    std::fs::read(format!("{dir}/match_s05.bin")).ok()
}

#[test]
fn match_s05_stroke_motions() {
    let Some(data) = load() else { return eprintln!("match_s05.bin absent, skipped") };
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
                assert_eq!((m, wait.is_some()), (m1, pending >= 0), "start k={k} p={p}");
                starts += 1;
            } else if now == 2 && was == 2 && m1 != m0 {
                if left == SWING_LEAD && p_i32(a, p, 0x3e40) >= 0 {
                    assert_eq!(m1, p_i32(a, p, 0x3e40), "switch k={k} p={p}");
                    switches += 1;
                } else if (0x27..=0x2a).contains(&m1) {
                    assert_eq!(whiff(m0), Some(m1), "whiff k={k} p={p}");
                    whiffs += 1;
                }
            }
            // the contact: countdown 1 → −1 and this player the last hitter
            if p_i32(a, p, 0x3ec4) == 1 && left == -1 && fr.global(0x423058) == p as i32 {
                let b = fr.live_ball();
                let v = [f(b, 0x140), f(b, 0x144), f(b, 0x148)];
                let soft = soft_follow(branch, p_i32(fr, p, 0x3e44), v, p_i32(fr, p, 0x3f50) as u32, 1.0);
                let m = p_i32(next, p, 0x3df0);
                match soft {
                    Some(s) => assert_eq!(m, s, "soft k={k} p={p}"),
                    None => assert!(!(0x1c..=0x1d).contains(&m), "no soft k={k} p={p}"),
                }
                softs += 1;
            }
        }
    }
    eprintln!("{starts} starts, {switches} switches, {whiffs} whiffs, {softs} contacts");
    assert!(starts > 100 && switches > 20 && whiffs > 3 && softs > 100);
}

#[test]
fn match_s05_serve_walk() {
    let Some(data) = load() else { return eprintln!("match_s05.bin absent, skipped") };
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
            assert_eq!(serve_walk(dx, fwd, 1.0), m, "k={k} p={p}");
            n += 1;
        }
    }
    assert!(n > 1000, "{n}");
}

/// Every post-point reaction: the outcome's base reaction or a team reaction of the character's set not taken
/// by a player updated before it (the draw itself is the game's random number).
#[test]
fn match_s05_reactions() {
    let Some(data) = load() else { return eprintln!("match_s05.bin absent, skipped") };
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
                assert!((0x2c..=0x2d).contains(&base) && team_reactions(CHARS[p]).contains(&c) && !taken.contains(&c), "k={k} p={p} {got:#x}");
                taken.push(c);
                team += 1;
            } else {
                assert_eq!(got, base, "k={k} p={p}");
            }
            n += 1;
        }
    }
    eprintln!("{n} reactions ({team} team)");
    assert!(n > 100 && team > 30);
}

/// The ANI sampler (squad rotations, Hermite positions, bone-length scale) against the players' skeletons in
/// five save-state RAM dumps (`context/ram/s0N.bin`): every multi-key track's quaternion, rotation rows and
/// position at the time last sampled, bit for bit. Character and costume are found by the motion's keys and
/// the skeleton's rest bones.
#[test]
fn clip_sampler_ram() {
    use hst_data::{ani, iso::Iso, mdl, xb::Archive};
    use hst_sim::pose::{Clip, Skeleton, q_matrix};
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
            // the character whose motion file has this clip's first rotation keys
            let first_rot = |k: usize| { let l = u(track(k) + 0xc); (0..list_n(l)).map(|j| [0, 4, 8, 12].map(|o| u(u(l + 0x18) + 16 * j + o) as u32)).collect::<Vec<_>>() };
            let want0 = first_rot(0);
            let nodes = u(u(u(u(an)) + 0xc) + 0x64);
            let node_of = |k: usize| nodes + ((u(u(clip + 0x1c) + 2 * (k & !1)) >> (16 * (k & 1))) as u16 as usize) * 0x120;
            let names: Vec<String> = (0..u(clip + 0xc)).map(|k| cstr(u(track(k) + 8))).collect();
            let rests: Vec<[u32; 3]> = (0..names.len()).map(|k| [0, 4, 8].map(|o| u(u(u(node_of(k) + 0x108) + 0x10) + 0x70 + o) as u32)).collect();
            let mut found = None;
            'search: for c in 0..16 {
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
                        found = Some((c, a, sk));
                        break 'search;
                    }
                }
            }
            let (c, a, sk) = found.unwrap_or_else(|| panic!("{s} p{p}: no character's motion {motion:#x} matches"));
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
                let ctx = format!("{s} p{p} char {c} motion {motion:#x} t {t} {name}");
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
/// of every reacting frame of the slot-5 match, bit-exact, with the motion time = frames since the reaction began
/// (held at the motion's length). The phase's last frames don't run the reaction (spot unchanged).
#[test]
fn match_s05_reaction_root() {
    use hst_data::{ani, iso::Iso, xb::Archive};
    use hst_sim::motion::reaction_root;
    use hst_sim::pose::Path;
    let Some(data) = load() else { return eprintln!("match_s05.bin absent, skipped") };
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
    for c in CHARS {
        let arc_data = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
        let arc = Archive::parse(&arc_data).unwrap();
        let get = |m: usize| { let stem = ani::motion_name(m, c as usize).unwrap().to_ascii_lowercase(); arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))).map(|e| ani::parse(&arc.read(e).unwrap()).unwrap()) };
        let a = get(0x2e).unwrap();
        gu_set.insert(c, (get(0x35).and_then(|d| Path::new(&d)), a.end_tick() as f32 / a.ticks_per_frame as f32));
    }
    let frames = frames_live(&data);
    let (mut exact, mut held, mut gu, mut t) = (0, 0, 0, [0f32; 4]);
    for k in 1..frames.len() {
        let (a, fr) = (frames[k - 1], frames[k]);
        for p in 0..4 {
            if p_u8(fr, p, 0x3fa4) != 2 || p_u8(fr, p, 0x3fa7) != 1 {
                continue;
            }
            if p_u8(a, p, 0x3fa7) != 1 {
                t[p] = 0.0;
            }
            let m = p_i32(fr, p, 0x3db0);
            let v = |f: Frame, o: usize| [0, 4, 8, 12].map(|d| f.player_f32(p, o + d));
            let (pt, team) = if m >= 0x30 {
                let k = (m - 0x30) as usize;
                (paths[k].as_ref().map_or([0.0; 4], |c| c.at(t[p].min(lens[k]))), true)
            } else if m == 0x2e {
                let (path, len) = &gu_set[&CHARS[p]];
                let Some(path) = path else { continue };
                gu += 1;
                (path.at(t[p].min(*len)), false)
            } else {
                continue;
            };
            t[p] += 1.0;
            let got = reaction_root(pt, team, CHARS[p], [v(fr, 0x3d40), v(fr, 0x3d50), v(fr, 0x3d60)], v(fr, 0x3d90), v(a, 0x3da0));
            let want = v(fr, 0x3da0);
            if got.map(f32::to_bits) == want.map(f32::to_bits) {
                exact += 1;
            } else {
                assert_eq!(want, v(a, 0x3da0), "k={k} p={p} motion {m:#x} t={}: {got:?}", t[p] - 1.0);
                held += 1;
            }
        }
    }
    eprintln!("{exact} reacting frames bit-exact ({gu} gu_set), {held} held at the phase's end");
    assert!(exact > 7000 && held < 50);
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
