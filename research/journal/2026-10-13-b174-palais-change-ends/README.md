# B174 — Grand Palais change of ends showed the inside of a wall

Status: FINAL.

## Finding

The port's change-ends glide (B49) ran from the post-point cut-away to the match camera, orbiting the server, with
the parts measured as eye − pivot and raw atan2 headings. With the camera on the human's side, the end view is on
the other side of the court (heading −π), so the glide went ~235° the long way round and the eye passed x ≈ −24 m,
outside the Grand Palais hall's side wall (roof underside, wall structure on screen).

What the original does (two recordings with `research/b174_swing_probe.py`: slot 5, a bot match, and a fresh Grand
Palais doubles match, P1 human + 3 COM, recorded to scratch slot 9; output in `context/b174/orig/`):

- The start view is **not** the view on screen. The change-ends set-up starts scripted camera shot 2 and copies
  its first view as the start (camera+0x100..0x140). Shot 2's subject is court frame 0x76: the player at the
  match camera's new end, on the ground and facing the net. In the Palais it was (−3, 0, 12.25) facing −z, and
  0x77 was the same spot mirrored, (3, 0, −12.25) facing +z. The start eye is behind that player:
  (−5.84, −0.14, 19.73) in the Palais and its mirror (5.84, −0.14, −19.73) in slot 5.
- The blend orbits that same 0x76 spot. The look's heading is a difference from the end view's, taken the
  shortest way round. The pivot's angles are as seen from the eye (pivot − eye).

## Fix

- `hst_sim::camera::swing`: the pivot's angles are now pivot − eye, and the heading takes the shortest way round.
- `play/change_ends.rs`:
  - The start is shot 2's first view (`cutaway::Shot::new(shots, 2, false)` built with frames 0x76 and 0x77).
  - The pivot is the player at the match camera's end.
- Fit (a Python model of `swing`, ticks after the first):
  - Slot 5: mean 0.10 m, worst 0.26 m.
  - Palais: mean 0.10 m, worst 0.24 m.
- Port vs original, Palais glide eyes:

  | Tick | Port | Original |
  |---|---|---|
  | 78 | (−5.84, −0.15, 19.78) | (−5.64, −0.02, 19.82) |
  | 40 | (−3.41, −7.14, 34.90) | (−3.72, −6.81, 34.53) |
  | 20 | (−0.43, −11.31, 38.82) | (−0.53, −11.22, 38.75) |

- Shot 2 with no mirror gives the start eye (−5.8356, −0.1447, 19.7292); the original's is (−5.8357, −0.1447, 19.7292).
- Screenshots at tick 40: `context/b174/orig/e40s.png` vs `context/b174/port/e40s.png` show the same framing,
  from behind the new baseline with the arched windows ahead.
- Test: `change_of_ends_glides_on_the_players_side` (stage 7, camera on player 0). Every tick's eye stays behind
  the new end's baseline and within |x| < 8 m. It fails on the old path, which reached x ≈ −24.

This also covers most of B164 (the glide's pivot and its end view).

## Not verified / not 1:1

- **Start view's yaw about 3° off.** The eye matches to 1e-4 m, but the original's start rows are turned about
  3° in yaw from the port's shot-2 build. Shot 2's framing yaw (the held-back yaw) at its first frame wasn't
  traced. It fades out over the glide and isn't visible by eye.
- **Which player 0x76 follows in doubles.** The port takes the first player at the camera's end. 0x76 read the
  human's spot in the Palais (−3 = P1) and 2.767 in slot 5 (a live position). With two players at that end, the
  one the game picks wasn't traced.
- **Plain f32 and musl, not `hst_sim::ps2`.** The camera never feeds back into play, and the rendering only has
  to look right.
- **The phase's first tick.** The original's first change-ends tick still shows the cut-away. The port shows the
  glide's first step from the shot-2 view, a one-frame difference.
