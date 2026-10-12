# 11 — Ball-hit obstacles (P14c4)

Types 14 (court 4), 21 (court 6), 38 (court 9) and 43 (court 10). 14 and 21 share one callback (its 38 branches are
dead code), and 38 and 43 each have their own.

## What they do (ported in `npc.rs`, message 2 arms)
- **14/21**:
  - An oriented-box test (separating axes: 6 face axes and 9 cross axes, ps2 ops) of the obstacle's world matrix
    with extents (1, 2.5, 0.1) / (1, 0.9, 0.025) against each player's model matrix (player+0x3d40), then the
    ball's (+0xb0). Every other box is a 0.2 cube.
  - The first box that overlaps sets the latch (+0x278).
  - A player, or a ball whose speed is at or over the row's km/h (30), restarts the animation and plays the sound
    (program 2, key 0xb / 0xa, volume int(115·1.2)).
  - Only the ball sends message 0x14 (`struck`).
- **38/43**: a ball-only axis-aligned test of the ball position ± 0.2 against the obstacle position ± (0.18, 0.8,
  0.18) / (0.15, 1.3, 0.15). Any overlap latches and sends 0x14.
  - 43 at or over its row's 20 km/h: animation and sound (program 7, key 0x23, ·1.2).
  - 38 at or over 20 km/h (a code constant): animation on clip 1 when at or over its row's 40 km/h, else on clip
    0 (`alt`, and `len` from `clip_lens`). It turns to the ball (yaw = heading of the unit vector to the ball,
    world = rot_x(pitch)·rot_y(yaw)). Sound: program 2, key 0xc, ·1.35.
- **Reset (0/4)**: on, home matrix, latch and animation cleared, pitch/yaw from home row 2, frame 0. The clip stays
  what it was.
- **Ball speed** = |+0x140|·60·3600/1000. +0x140 is the ball's **last move** (a position delta), not its velocity
  (+0x130).
- Rows: `exe::TriggerRow::hit_kmh`. The extents are code-initialized statics (zero in the ELF, read from RAM).

## Proof
- Oracle (`crates/hst-oracle/tests/npc.rs`):
  - `obstacle_boxes`: 30k random matrix pairs × types 14/21/38 on `s03.bin` RAM, 10.8k overlaps, bit-exact verdict.
  - `ball_speed`: 20k random moves, f0 bit-exact.
- Fixtures (context/fixtures, `HST_BALL=1 HST_MATS=1 tools/record_npc.py … prox`; new `B` poke target = ball
  position + its last move):
  - `hit_c04` (slot 3, pad-driven serve):
    - creature 0 poked onto a player: animation, no 0x14;
    - creature 1 overlapped the server during the toss: animation, no 0x14;
    - creature 2 at 66 km/h: animation + 0x14;
    - creature 3 at 1.5 km/h: latch + 0x14 only;
    - the reset clears the latch.
  - `hit_c09` (slot 8, court 9):
    - creatures 2 (92 km/h) and 3 (42 km/h): clip 1 and the turn, bit for bit;
    - creatures 1, 4 and 5 (slow): latch + 0x14 only.
  - `hit_c10`: 5 slow hits (latch + 0x14 only).
  - `hit_c10b`: 2 fast hits (animation).
- `ball_hit_obstacles_match_the_game` steps every tick of every obstacle in the 4 files and compares the whole
  `Trigger`. It checks struck ⇒ the tick's message 0x14, every new 0x14 coming from a struck obstacle, a sound with
  each animation start, and the reset's fields. 14 hits, 7 with the animation.
- What the callbacks see is the state **before** the game's tick: taking the boxes, the ball position or the km/h
  from the sample after fails each time. In play the creature step runs after the ball step. That is the same
  input the game's creatures get at the start of the next tick (pre-(N+1) = post-N). The 0x14 step off the
  obstacle (`step_plane`) also lands before the ball's next own step either way, so play needs no reorder.
- One torn real-time sample (`hit_c10b` vsync 7854, an animating frame read one tick late: 19, 21, 21) is skipped
  and counted. The test allows at most 2.

## Play (`play/npcs.rs`)
- `Near::boxes`:
  - each player's matrix is built from `body.face.dir` (rows (-d.z,0,d.x), up, d, pos). Only the absolute value
    of the side row was checked against RAM, and the box test doesn't depend on its sign;
  - the ball's is an unrotated box at its position.
- `Near::kmh` = `ball_kmh(pos − last tick's pos)`.
- Sounds go through `npc::trigger_sound`.
- Type 38 draws clip 1 after a fast ball, and its `clip_lens` come from its model's motions 0/1.
- `struck` → `step_plane` + doubles AI 0x14 (already there).

## Not verified / not 1:1
- **Ball box rotation**: the game's ball matrix (+0xb0) carries the drawn ball's spin. Play uses an unrotated box,
  so edge grazes of 14/21 can differ. → P14c4a.
- **Km/h while the ball stands between steps**: the original keeps the last move; play gives 0. This only gates
  animation, sound and 38's turn. → P14c4a.
- **Type 21 is unrecorded** (no court 6 state). It shares 14's code; its extents are covered by the oracle. Its
  key 0xa and threshold come from the code and the exe only. → P14c4b.
- **Sounds**: these types don't write a sound handle (+0xc4 stays −1), so key and volume come from the code only.
  The test checks only that a sound starts with each animation. This is presentation (by eye, PLAN), so no task.
- **The players' matrix side-row sign** wasn't matched. It is irrelevant to the overlap verdict, and nothing else
  reads it here.
- **The reset's route fields** (`left`, `from`, `target`, `to`) for these path-less obstacles differ from RAM
  (generic reset; never read for these types). Not compared.
- **When the latch clears**: play still clears it at the next point (the existing ponytail, P14c3 leftovers). In
  `hit_c04` the game's reset cleared it at a message 0xc/0xe/0x1a, and the test checks that reset.
