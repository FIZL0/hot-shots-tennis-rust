//! AI decisions against `hst_sim::ai`, through the oracle. Needs `context/iso/` and `context/ram/s05.bin`
//! (skips without them).

use hst_oracle::{Ee, addr};
use hst_sim::ai::Runner;
use hst_sim::player::Stats;

fn rng(seed: &mut u64) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 32) as u32
}
fn uni(seed: &mut u64, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * (rng(seed) as f32 / u32::MAX as f32)
}
fn int(seed: &mut u64, lo: u32, hi: u32) -> i32 {
    (lo + rng(seed) % (hi - lo + 1)) as i32
}

/// The run estimate over random players and spots: a player block in free RAM (offsets as
/// `tools/record_ai_search.py` reads them), the match globals as the slot-5 dump has them, rewritten per call.
#[test]
fn run_frames() {
    let Some(mut ee) = Ee::from_ram("s05.bin") else { return eprintln!("skip: no context/iso or context/ram/s05.bin") };
    let (at, p, spot, gm) = (addr("ai_run_frames"), 0x01e0_0000u32, 0x01e1_0000u32, ee.read(0x42_2f80, 4) as u32);
    let (mut seed, mut bad, n) = (0x2545_f491_4f6c_dd1du64, vec![], 20_000);
    let mut found = 0;
    for _ in 0..n {
        let s = &mut seed;
        let r = Runner {
            stats: Stats { speed: uni(s, 0.3, 1.2), agility: int(s, 1, 40), stamina: int(s, 5, 30), ..Default::default() },
            size: [100, 100, 80, 70, int(s, 50, 150)][int(s, 0, 4) as usize],
            side: if rng(s) & 1 == 0 { 1.0 } else { -1.0 },
            pos: [uni(s, -9.0, 9.0), uni(s, -18.0, 18.0)],
            stamina: int(s, 0, 30),
            tick: int(s, 0, 59),
            run: int(s, 0, 40),
            moving: int(s, 0, 3) as u8,
            players: int(s, 1, 4),
            rally: rng(s) & 1 == 0,
            floor: int(s, 0, 9),
        };
        let to = [uni(s, -10.0, 10.0), uni(s, -20.0, 20.0)];
        let f = |v: f32| v.to_bits() as u128;
        for (o, v) in [
            (0x12b0, f(r.side)),
            (0x12c8, r.size as u128),
            (0x1374, f(r.stats.speed)),
            (0x1378, r.stats.stamina as u128),
            (0x1388, r.stats.agility as u128),
            (0x3d70, f(r.pos[0])),
            (0x3d78, f(r.pos[1])),
            (0x3df4, r.stamina as u128),
            (0x3df8, r.tick as u128),
            (0x3dfc, r.run as u128),
        ] {
            ee.write(p + o, 4, v);
        }
        ee.write(p + 0x3fa5, 1, r.moving as u128);
        ee.write(0x42_2fa4, 4, r.players as u128);
        ee.write(gm + 0x55, 1, if r.rally { 3 } else { 2 });
        ee.write(0x3f_c8d8, 4, r.floor as u128);
        ee.write(spot, 4, f(to[0]));
        ee.write(spot + 8, 4, f(to[1]));
        ee.call(at, &[p as u64, spot as u64], &[], 1_000_000);
        let (want, got) = (ee.v0() as i32, r.frames_to(to));
        found += (want != 9999) as usize;
        if want != got {
            bad.push(format!("{r:?} to {to:?}: game {want}, port {got}"));
        }
    }
    eprintln!("{found} of {n} spots reachable");
    assert!(found > n / 10, "too few reachable spots ({found}) to say much");
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(10)].join("\n"));
}
