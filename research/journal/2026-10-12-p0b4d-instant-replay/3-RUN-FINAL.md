# P0b4d2: the replay run (`play/instant_replay.rs` `run`)

The original's serve-phase entry (`324b50`, message 0xe) saves the match globals, the rally block and the RNG seed
when no replay is pending. With one pending it restores them, sets gm+0x344, counts gm+0x358, and the point plays
again from the recorded pads. The port does the same with M2c's `play/snapshot.rs`:

- `next_point` → `instant_replay::new_point` sets `Instant::set_up`. `run` is an exclusive system at the end of
  each FixedUpdate tick (after `play_sounds`). At a set-up it `snapshot::save`s the match (`Game` with every
  generator and the score, the pads, the rigs' motions, the modules' state) and clears the pad log. On other ticks
  it logs `TickPads::now()`. On the match's first tick, which has no `next_point`, it saves too.
- At a set-up with `decision.replay` set and not replaying, `start` restores the kept state, carries the count over
  (`replays` +1; `asked` stays, P0b4d5), sets `replaying`, and queues the logged pads as `tick_input::Replay`. An
  outside replay source's queue is set aside and comes back after.
- The point, its post-point phase and the next serve set-up then play again. The scoreboard awards the point onto
  the score from before it, so the point isn't scored twice. `Point::replaying` stops a second decision. At that
  set-up `run` clears `replaying`, saves again, and gives the input back.
- The match-ending point replays the same way: the port's match-over path runs `next_point`, so the replay plays
  and the match ends a second time, then the new match starts.
- `cutaway::PickInput::replays` = `g.instant.replays`, so the long post-point shots come back on game-ending points
  after a replay.

## Check

`play::instant_replay::tests::the_replay_ends_where_the_point_left_the_match` runs a bot-only singles match headless.
During the replay the score is put back to before the match point and the count is 1. The replay decides the match
again. At the replay's closing serve set-up, `Game` (Debug text, the `instant` block aside) is identical to its state
at the set-up that started the replay. `a_bot_match_replays_its_match_point_only` still passes with `run` in its
schedule. Both ~2 s in release.

## Not verified / not 1:1

- **Not watched against the original.** It replays at normal speed with the match camera. The original's
  fast-forward/slow-mo windows (P0b4d3) and replay cameras (P0b4d4) aren't ported, so it doesn't look like the
  original's replay yet. No PCSX2 side-by-side was done for the same reason.
- **The port replays the post-point phase too.** The original restores at the serve entry and "returns to the
  post-point phase"; whether its post-point phase after a replay is exactly the first one (umpire call, cut-away,
  B48's held camera on match point) is P0b4d4's to check. The sim state at the end is the same either way.
- **The replay adds its own ticks of match time**, as the original does. A PCSX2 recording through a replay would
  need P0b4d3's schedule to stay frame-aligned.
- **Presses during the replay** latch in `Pads` and land after it. P0b4d5 decides what a press does then.
- **`asked`** (gm+0x354) is never counted: nothing asks by button yet (P0b4d5).
- **The every-third-replay role swap** in `cutaway::roles` is still not ported (P0b4d4).
- **A new `Game` mid-run** (a second match started by re-running setup) would keep the last match's serve state
  until its first set-up. The app inserts `Game` once, so nothing hits this today.
