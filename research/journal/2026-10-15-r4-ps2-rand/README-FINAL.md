# R4 — newlib rand() and MT19937 split out (`hst_sim::ps2_rand`)

- `crates/hst-sim/src/ps2_rand.rs`: `Rand` (newlib `rand()`, `Rand::new` = `srand`) and `Mt` (`init_genrand`,
  `genrand_int32`), no imports from the game modules (only serde; its own 624-word serde helper instead of
  `crate::big`), so R6 can lift it into ps2-float as is.
- `rng.rs` re-exports both (`hst_sim::rng::{Rand, Mt}` and `weather::…` paths unchanged) and keeps the game's side:
  `Mt::from_ram` (the game's 0x9d0-byte object layout), `r15`, `bit`, `unit`, then `Seed`, `Rngs` and the rest.
  These are an inherent `impl Mt` in rng.rs (same crate); when R6 moves `Mt` to its own crate they become a trait.
- `crates/hst-sim/src/mt.rs` deleted: an unused second copy of MT19937 (nothing referenced `hst_sim::mt`).
- Oracle (`crates/hst-oracle/tests/rng.rs`, names in `research/oracle.txt`): the boot program's `rand()` over 20k
  64-bit states (incl. 0, 1, max): result and stepped state; the game's MT seed over 10k seeds (incl. 0, 1, 5489,
  max): all 624 words + index; the draw over 12k random states at every index 0..626 (so the regeneration path
  runs ~57 times): result + whole state. All bit-exact.
- `hst-oracle` now builds at opt-level 3 in dev (40 s → 4 s for the MT check).

## Not verified / not 1:1
none — both generators are integer-only and match the game's own code over the oracle inputs above.
