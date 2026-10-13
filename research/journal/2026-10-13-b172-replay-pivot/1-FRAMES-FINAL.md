# B172: replay cameras aimed at the wrong frames (FINAL)

## Cause
The replay's frame table (`replay_camera.rs show`) was guessed from how the shots use it (P0b4d4). Several picks
got the wrong subject:
- 0x06, the first tracking shot, has subject 120. In the original that frame stays at identity during replays,
  i.e. the court centre. The port used the ball, so 0x06 orbited the ball.
- 0x08 uses 113/114. Those are identity too, not the role bodies.
- The 0x23 family has subject 56, and 0x38/0x26/0x31/0x4d have subject 88. These are the coming hit's recorded
  head bone and pelvis, rebuilt on the hitter's live body. The port used the other player's chest.
- The close-ups 0x56/0x58 look at frame 8, the ball. The port had identity there.
- 0x53/0x54/0x55/0x5a/0x5b use frame 11, the last bounce. The port used the live ball.
- Frames 5, 7, 14, 17, 24, 25, 42 and 57 were identity or the wrong player's frame.

## What the original keeps (decompile, then checked against its RAM)
- **Live per player.**
  - Body: the anim root with y = 0.
  - Head frame: the port's `cutaway::head_frame`.
  - Head bone: the head frame without its 0.2 m lift.
  - Spine2 and Pelvis.
- **Copies in the table.**
  - Roles a (last hitter) and b (the one before): 19/20, 35/36, 51/52, 67/68, 83/84.
  - The same roles as at the shot's start: 21/22, 37/38, ….
- **Recorded at each hit**, for the hitter: the forward direction, and the position in the hitter's body frame, of
  Spine2 on the ground, head frame, head bone, Spine2 and Pelvis.
  - The replay rebuilds each one upright along that forward direction, on the subject's live body.
  - 23/39/55/71/87 come from the coming record.
  - 24/40/56/72/88 come from its copy at the shot's start.
  - The subject is the coming record's hitter. Past the last shot it's the player before the last hitter.
  - 25/41/57/73/89 are the subject's live frames; 26/42/58/74/90 are the next hitter's.
- **Ball.**
  - 8 (also 9, 10 and 12): along the ball's velocity, at the ball.
  - 14: along the ball at the coming hit's spot, set each time a replayed hit passes.
  - 17: frame 14 as it was at the shot's start.
- **11:** the live point's last bounce, on the ground, facing along the ball. It's turned round to face from a's
  side to b's.
- **Court spots.**
  - 5: the service-line spot on a's half, turned round.
  - 7: the net top over the centre (0.91 m).
  - 1, 113, 114, 120 and the rest: identity.

## Check
- `research/b172_frames.py` recorded slot 5's match-point replay into `context/b172/run1`, `run2` and
  `context/fixtures/replay_frames_s05.bin`. Each sample has the table, the recorder's role words and the
  camera's eye and look.
  - Frame by frame, every frame above matches the rule written here. Examples: 120, 113 and 114 at identity; 5 at
    ±6.4 on a's half; 17 = 14 copied at the cut; 24's y = 0; 56 near the subject's head as recorded; 25 = the
    subject's live body.
- New test `hst-sim/tests/replay_camera.rs` feeds the port's `Shot` the recorded table.
  - 0x38: the eye is within 2 mm of the game's.
  - 0x53: the look direction is within 0.05 of the game's.
- `play::replay_camera` test: the bot match's replay still cuts 0x06 → 0x1d → 0x06 → 0x38 → 0x53.

## Not verified / not 1:1
- **No `--shot` mid-replay.** Replays are off in the app (`instant_replay.rs REPLAYS`, until B127 is fixed), so
  no remaster screenshot of a replay can be taken. The check is numeric instead, against the game's own camera.
  That is stronger than by eye, but only for the shots that replay showed (0x38, 0x53).
- **0x53's eye is ~0.8–1.2 m from the game's**, even fed the game's own frames. The view direction matches, and
  the game's eye is steadier. The cause is in `Shot` (shared with the cut-aways), not in the frames. Not chased:
  it looks the same by eye.
- **0x38's look drifts** in the game (x 0.31 → 0.16 over the shot) and only partly in the port (look error up to
  0.16). This is the record's target-follow, also in `Shot`.
- **The rebuilt frames use the port's own frames, not the game's.** The bones come from the port's skeleton and
  the body from its face direction, so they match only as closely as the port's pose. The flip of the body's rows
  for a mirrored model isn't applied to Spine2/Pelvis (only their forward and position are used).
- **Frame 11's turn-round** keeps the bounce spot. The decompile multiplies by a rotation whose side wasn't
  checked, and the game showed it unturned here. The game's condition for which bounce counts (match phase 3–4)
  is taken as any bounce of the live point.
- **Frames 1 and 7 are never turned round.** The game turns them when a side flag (not traced) is set.
- **Frame 15** (the hit after next, used only with more than 2 players) isn't kept.
- **Frame 10's switch** (it stops following the ball once the ball has rolled) isn't kept: 10 is the ball.
