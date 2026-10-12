# M2h1 — instant replays in lockstep and in `replays_a_desync`

## The bug

M2g's gate (`netplay.rs gate`) put each tick's synced pads at the back of `tick_input::Replay`. During an instant
replay that resource is the replay's own queue (`instant_replay::start`). So each network tick's pads were queued
behind the replay's pads. One was pushed and one was popped every tick, so the queue never ran out. Once the
recorded pads were used up, the replay played the live pads instead. When the replay ended at the next serve
set-up, `end_queue` dropped the rest, and those ticks' live pads were lost. The replay's ✕/○/△/□ check
(`instant_replay::presses`) read this machine's `Pads` latches, not the synced pads.

Every machine went wrong the same way, so the state hashes still agreed and the M2g tests passed.

## Fix

`netplay::run_on(w, now, last)` is shared by the lockstep gate and `replays_a_desync`. Outside a replay it pushes the
tick's pads as before. While `instant_replay::playing`, it leaves the queue alone. It sets every seat's
`Pads` shot/serve latches to the tick's new presses (`tick_input::decode(now, last)`) instead, so they are the same
on every machine, and this machine's unsent latches are dropped. Rollback's `handle` already works this way.
`replays_a_desync` no longer replaces `Replay` every tick (that cut an instant replay's queue down to one tick).

## Verified (`netplay.rs` tests, `matches`)

- The headless harness now runs `instant_replay::run` at the end of each tick, as the app does. Before, it had no
  instant replays.
- `doubles_at_100ms_stays_in_sync` (lockstep, 3600 ticks, mashed pads): in sync, 18 instant replays. `play()` checks
  that while one replay plays, its queue shrinks by one every tick. With the old gate restored, this fails at
  replay 1 ("a network tick's pads went into instant replay 1's queue").
- `saved_inputs_replay_the_same` (new): the host's lockstep log from 2400 rounds, run again as `replays_a_desync` does
  (including the instant replays in it), gives the same `state_hash` as the host.
- `rollback_at_150ms_stays_in_sync`: in sync with 4 instant replays in 1132 ticks. It now asserts that more than
  one replay happened.
- The full-match tests (`full_doubles_match_at_100ms…`, `full_doubles_match_rollback…`, ignored) run without
  instant replays (`REPLAYS` thread-local in the harness). With them on, the mashed pads keep asking for another replay
  (△ during one), and the rollback match hadn't finished after 400k ticks. Without them, both pass in release:
  rollback in sync over 12301 ticks to the result (31 s), lockstep in sync to the result (195 s).

## Not verified / not 1:1

- **The skip's fast-forward in the headless harness**: `buttons`' skip adds to `Time<Fixed>`'s overstep, and the
  harness runs one tick per round regardless. So the harness checks that the synced presses reach the replay's
  button check (△ asks for replay 2 straight after replay 1), but not that ✕/○ shortens the replay. In the real app,
  every machine adds the same skip, because the presses are synced.
- **A full match with instant replays**: never reaches the result with mashed pads (above), so a whole match with
  replays is checked only in parts: 3600 lockstep ticks and 1200 rollback rounds.
- **The real two-process app with lockstep**: not run. The headless lockstep harness uses the same gate and tick
  schedule.
- **Original game**: it has no online play, so there is nothing to match 1:1.
