# B494 — rollback test harness independent of round length

## Problem
`rollback_desync_is_saved_and_play_goes_on` failed twice during P60 (2 resyncs, 1 expected) while other builds were
loading the machine. `play_rollback_with` runs the two peers in rounds and delays each datagram by `lag` rounds, but
ggrs measured time with the real clock: the ping (frame advantage → `gate`'s time sync shift), its keep-alive,
retry and quality-report intervals, and `network_stats`' first-second `NotEnoughData`. `rollback.rs`' B198 `resend` used
the real clock too. So a slow round changed the traffic, the time sync and the resends.

I couldn't make the 2-resync failure happen on purpose: 12 parallel copies of the test, 4 parallel copies of the whole
netplay set, and sleeps of 5–300 ms on some rounds all passed. The outcome did move with timing, though: where the
resync landed (`fresh` 339–342) and the last tick hashed (840/870) changed with the sleep pattern.

A second source turned up while writing the determinism test: two runs with no sleeps at all also differed. They
differed from tick 2 on. `rollback::advance` read the local seats in `ggrs::P2PSession::local_player_handles()` order,
which comes from iterating a HashMap and so changes from run to run. The harness's mashed pads come from one
generator per call (`Netplay::mashed`), so the host's two seats swapped presses and every run played a different
match. Real devices weren't affected: each seat reads its own pad slot. This seat-order bug is a more likely cause of
P60's flake than load, and before the fix 6/8 runs of the new test failed on it.

## Fix
- `third_party/ggrs` (a patched copy of ggrs 0.13.0, in `[patch.crates-io]`): new `ggrs::clock`. It returns real time
  unless the calling thread drives it (`drive(true)`, `advance(d)`). Everything in `network/protocol.rs` now reads
  this clock: `Instant::now()` and `millis_since_epoch()`. Left on real time: the lockstep spin-wait in
  `advance_frame_with_wait_timeout`, which we never use, and which would never end on a driven clock. The examples,
  tests and dev-dependencies were left out of the copy.
- `rollback.rs`: `resend`'s stall clock reads `ggrs::clock::now()`; `advance` sorts the local handles before reading
  pads.
- `netplay.rs` tests: `play_rollback_with` drives the clock and moves it 1/60 s per round (one tick of time).
- New test `rollback_rounds_play_the_same_however_long_they_take` (~6 s). It plays the forced desync's match to round
  420 twice, once as fast as possible and once sleeping 10–49 ms every 9th round. It asserts that both peers' pad logs
  and their state tick, `fresh`, resyncs (1), frame, re-runs, sat-out, held and resent counts are identical.

## Checks
- New test: 8/8 parallel runs pass (6/8 failed before the handle sort). Under load (12 copies at once, ~35–42 s each
  instead of ~6), 12/12 runs of it and `rollback_desync_is_saved_and_play_goes_on` pass.
- `HST_NETPLAY=1 tools/check.sh -p hst netplay` 27/27 and `… rollback` 5/5 pass.
- `cargo check -p hst-base` and `-p hst` for wasm32 build with the patched ggrs.

## Not verified / not 1:1
- I couldn't reproduce the original 2-resync failure, so I can't show that it was load rather than the random seat
  order. Both are fixed, and the new test proves the outcome no longer depends on either.
- `tabbed_out` (B162) and the recover tests' own harnesses still use real time on purpose: they model a real-time
  absence (`away`). Not changed.
- No gameplay change and no PCSX2 comparison needed. In the game, the clock thread-local is never driven, so ggrs
  behaves as before. The handle sort only changes the order in which local pads are read.
