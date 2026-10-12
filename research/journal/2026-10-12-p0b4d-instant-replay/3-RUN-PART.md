# P0b4d2: the replay run — plan (in progress, nothing coded yet)

Stopped at a usage limit after reading the code; this is the plan to pick up from.

## Design

- Reuse M2c's `play/snapshot.rs` (`save`/`restore` cover `Game`, pads, `TickPads`, motions and the modules'
  resources; its test shows a restore + the same pads re-runs bit for bit).
- New exclusive system in `play/instant_replay.rs`, after the FixedUpdate chain (after `held_ball`):
  - `next_point` already calls `instant_replay::new_point`; have it set a flag on `g.instant` ("serve set up").
  - On the flag, with no replay pending: keep `snapshot::save` as the point's serve snapshot, clear the pad log,
    clear `replaying`.
  - On the flag with `g.instant.decision.replay`: restore the serve snapshot, then put back the replay counters
    (+1 to `replays`, gm+0x358; +0x354 only for a button-asked one, P0b4d5) and set `replaying`; insert
    `tick_input::Replay` with the logged pads so the point, its post-point phase and the next serve set-up play
    again from the recorded pads.
  - Every tick log `TickPads`' now-pads (needs a small accessor in `tick_input.rs`).
- No double score / RNG: the restore puts back the score and every generator (all in `Game`), and the rerun is
  deterministic, so the state at the second serve set-up equals the first; `Point::replaying = g.instant.replaying`
  stops a second decision. Same as the original, which restores 0x422f90.. and the seed and plays on.
- Match point: the port's MatchOver path calls `next_point`, so the replay runs there too and the match ends again.
- `cutaway::Inputs::replays` (`play/cutaway.rs:128`, now 0) ← `g.instant.replays`.
- Remove the `Replay` resource when it runs dry (else `local` input never comes back).

## Check to write
Extend `a_bot_match_replays_its_match_point_only`: state hash at the serve set-up after the replay equals the hash
at the first one (replay counter aside), and the replay count is 1.
