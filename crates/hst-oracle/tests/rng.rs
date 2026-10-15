//! newlib's `rand()` and the MT19937 against `hst_sim::ps2_rand`, through the oracle. Needs `context/iso/` (skips
//! without it).

use hst_oracle::{Ee, addr};
use hst_sim::ps2_rand::{Mt, Rand};

fn rng(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

/// `rand()` from random 64-bit states: the result and the state it leaves.
#[test]
fn newlib_rand() {
    let Some(mut ee) = Ee::game() else { return eprintln!("skip: no context/iso") };
    let (at, state) = (addr("newlib_rand"), ee.read(addr("newlib_impure_ptr"), 4) as u32 + 0xa8);
    assert_ne!(state, 0xa8, "impure pointer unset");
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    for k in 0..20_000 {
        let s = match k {
            0 => 1,
            1 => 0,
            2 => u64::MAX,
            _ => rng(&mut seed),
        };
        let mut r = Rand(s);
        ee.write(state, 8, s as u128);
        ee.call(at, &[], &[], 1000);
        assert_eq!((ee.v0() as u32, ee.read(state, 8) as u64), (r.next(), r.0), "state {s:#018x}");
    }
}

const OBJ: u32 = 0x01e0_0000;

fn state(ee: &Ee) -> Mt {
    Mt(std::array::from_fn(|k| ee.read(OBJ + 4 + 4 * k as u32, 4) as u32), ee.read(OBJ + 0x9c4, 4) as usize)
}

fn put(ee: &mut Ee, m: &Mt) {
    for (k, &w) in m.0.iter().enumerate() {
        ee.write(OBJ + 4 + 4 * k as u32, 4, w as u128);
    }
    ee.write(OBJ + 0x9c4, 4, m.1 as u128);
}

/// Seeding from random seeds gives the same 624 words and index; then draws from random states at every index
/// (624 and past it: the regeneration) match result and state.
#[test]
fn mt19937() {
    let Some(mut ee) = Ee::game() else { return eprintln!("skip: no context/iso") };
    let (seed_at, next_at) = (addr("mt_seed"), addr("mt_next"));
    ee.write(OBJ + 0x9c8, 4, 0);
    ee.write(OBJ + 0x9cc, 4, 0x9908_b0df);
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    for k in 0..10_000 {
        let s = match k {
            0 => 0,
            1 => 1,
            2 => 5489,
            3 => u32::MAX,
            _ => rng(&mut seed) as u32,
        };
        ee.write(OBJ + 0x9c4, 4, 0x271);
        ee.call(seed_at, &[OBJ as u64, s as u64], &[], 100_000);
        assert_eq!(state(&ee), Mt::new(s), "seed {s:#x}");
    }
    for k in 0..12_000 {
        let mut m = Mt(std::array::from_fn(|_| rng(&mut seed) as u32), k % 627);
        put(&mut ee, &m);
        ee.call(next_at, &[OBJ as u64], &[], 100_000);
        assert_eq!((ee.v0() as u32, state(&ee)), (m.next(), m.clone()), "draw {k}");
    }
}
