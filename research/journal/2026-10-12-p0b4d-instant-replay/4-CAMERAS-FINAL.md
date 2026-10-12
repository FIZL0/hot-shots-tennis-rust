# P0b4d4: instant replay cameras

## What the original does (read from the decompile)

- **Shot recorder**: every hit appends a record with its tick, hitter, type (the ball's class; 6 when the hitter is
  diving, 4 for the class-3 sub-kind-1 smash), kind and lead (ticks from the swing to the hit). Bot shots are always
  kind 0. When the point ends after more than one hit, the last record becomes kind 3 (6 if it was a smash) if its
  hitter is the point's player. A point-end record follows it.
- **Replay director**: each replay counts up from −1. It opens on a tracking shot: a pair table indexed by
  replay % 5, a per-pair counter, and 8→9 with more than 2 players. The serve record is picked 10 ticks before the
  serve. After each replayed hit the next record is scheduled if its kind isn't 0: at its tick − lead (kinds
  1/2/3/6 with a lead), else at its tick + the kind's offset. At the pick tick the shot comes from the per-type list
  (skipping some by costume or character), and the kind's window ends it. When the window closes it goes back to
  tracking, unless the last record is done (or the point ends within 4 ticks), in which case it goes to the
  closing shot: the close-up set, the ponytail/special swaps, 0x56 not after 0x47. That shot holds.
- **Lists, tables and windows** are retail data and are read from GAME.BIN (`exe::Game::replay_cameras`).
- **Role swap**: during replays 3, 6, … the post-point roles are swapped again unless the other player dove
  (`cutaway::roles`'s new `replays`/`diving` arguments).
- `reset_cut` (a replayed close line call) resets the closing-list counter.

## Port

- `hst-sim/src/cutaway.rs`: `ReplayPicks` (`shot`, `tracking`, `end`) and `Look`.
- `hst/src/play/replay_camera.rs`: records live shots, drives the director while `g.instant.replaying`, and builds
  the frame table from the two role players (body, head frame, head bone, chest) and the ball. It is not part of the
  snapshot, so its counters keep running across replays, as in the game.
- `play/cutaway.rs`: no cut-away runs while a replay plays.
- Test `play::replay_camera::tests::the_match_point_replay_cuts_and_holds_its_closing_shot`: the bot match's
  match-ending replay cuts 0x06 → 0x1d → 0x06 → 0x38 → 0x53, and no cut-away shows during or after it. B48 saw the
  same set on a match-ending point (0x1d, 0x06, 0x38, 0x53).

## Not verified / not 1:1

- **No PCSX2 side-by-side.** The B48 recordings are gone from `context/b48/`, and the shot sequence is checked only
  against B48's notes. Framing was not compared by eye.
- **Frame table approximated.** These frames are mapped by eye from how the shots use them, not traced:
  - 56/57/88/112–114/120 come from the per-record frames the recorder saves. Here they are the role players' bodies
    or chests, or the ball.
  - 11/12 are the ball.
  - 17 is the hitter's body.
  - Anything else is identity.
- **Kinds for human shots stay 0.** The game marks a human's strong shots kind 1 (from a character stat and the
  ball's speed). That wasn't traced, so a human-won rally's mid-rally strong shots get no camera of their own.
- **Lead = the port's swing start** (`Player::swing` becoming Some), not the game's own swing tick field.
- **"Point player"** (whose last shot becomes kind 3) is taken as "the last hitter's team won the point".
- **Two flags** in the costume/character skip checks were treated as false. They're the ones that pick 93/99, as
  in B48.
- **Mirror is always false** for replay shots.
- **The closing shot doesn't hold into match over.** The port has no match-over phase (`Next::MatchOver` starts
  the next match), so the match camera takes over when the replay ends.
- **`reset_cut`** is applied at replay start. Its real use is the fast-forwarded replay (P0b4d3), which isn't in
  yet.
- **Cut-away suppression during non-match-point replays** isn't verified: bot matches only replay match point.
