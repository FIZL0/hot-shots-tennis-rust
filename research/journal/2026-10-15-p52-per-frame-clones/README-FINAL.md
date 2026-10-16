# P52 — per-frame clones in the app

- `play/main_menu/online.rs`: `align`/`settle` took a deep copy of the lobby `Room` every frame. Now the room is
  `take()`n out of `net.room`, the body (`align_to`/`settle_to`, unchanged) runs on `&Room`, and it is put back;
  neither body reads `net.room`.
- `effects.rs` `morph_on_cpu`: no `base.clone()`, no `collect` into a new attribute and `last` copied in place when the
  target count matches. Positions are written straight into the mesh's `Float32x3` position attribute: per vertex
  base, then `+ d * k` per nonzero target in target order (same ops, same order as before). The existing
  `morph_on_cpu` test still gives the same positions.
- `play/recover.rs` `tick`: `told` borrowed instead of cloned; host/peer silence counters looked up by `&str`
  (`quiet()` copies the name only on first insert); peers iterated as `&str` (consecutive duplicates skipped as the old
  `dedup` did), a name copied only when pushed onto `away`. New `Recover::all_loaded` replaces the per-tick
  `loading(..).is_empty()` (which built a `Vec<String>`).
- `play/netplay.rs`: `Lockstep::advance` and `gate` lend `mine` out with `mem::take` and put it back instead of cloning;
  `loaded` copies `me` only when inserting it.

Tests: `tools/check.sh -p hst` (442 passed), `a_silent_peer_rejoins_and_a_silent_host_ends_it --ignored` passed.

## Not verified / not 1:1
- No frame-time measurement before/after: the changes only remove allocations, no behaviour change; the existing tests
  (lobby, morph, recovery, lockstep) cover the behaviour. Nothing gameplay-side touched, so no PCSX2 comparison needed.
