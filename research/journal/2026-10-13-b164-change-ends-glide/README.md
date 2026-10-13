# B164: the change-ends camera glide, ported from the game's routine

Status: FINAL.

## Where B164 stood

B174 (2026-10-13) had already fixed the user's complaint. Before it, the mid-glide view sat low behind the net. B174 changed:
- the start: scripted shot 2's first view;
- the pivot: court spot 0x76, the player at the match camera's new end;
- the heading: it now turns the short way round.

That covers both the solo-human 180° turn (Grand Palais, camera on the human's side) and the bot match. But
`hst_sim::camera::swing` was still a fitted model. It decomposed each view into absolute heading and pitch plus the
pivot's angles in the heading frame, and it missed the recordings by up to 0.26 m. This was most visible on the first
tick, and the start's look was about 3° off in yaw.

## What the original does

The change-ends branch of the camera handler, run each tick while its 80-tick counter is armed:
- **Reference frame R:** the rotation of this tick's shown view (the per-tick camera object's output), placed at the
  position of the court frame the handler names (0x76).
- **Decompose:** both the saved start view and this tick's view are taken apart in R by the game's generic
  orbit decomposer, the inverse of the builder that scripted shots use. The parts are:
  - the look's pitch and yaw as wrapped differences from R's, each `(a+π)−(b+π)` with the ±2π pick;
  - the look's roll;
  - with d = R.pos − eye in the view's own axes: yaw = atan2(x, z) and a pitch term atan(tan(pitch)/cos(yaw)), where
    pitch = atan2(−y, hypot(x, z));
  - |d|, a constant 1, and the fov (half-angle). The second-subject fields are zeroed.
- **Blend:** the nine parts are blended as `(cur − start)·w + start` with the B49 weight.
- **Rebuild:** the view is rebuilt by the shot builder, R × rotY(yaw) × rotX(pitch). It is then re-made upright from
  its forward axis, and the eye = R.pos − (axes × rotY(ey) × rotX(atan(tan(ep)·cos(ey)))).z · |d|. This is the start of
  `cutaway::Shot::build` with R as the basis.
- **Shown result:** it is written back as the shown view, with fov in full degrees.

The pitch and yaw differences are re-applied as local rotations in a pitched frame. That is why the first tick
doesn't show the start view exactly, and where B174's "3° yaw" came from.

## Port

- `hst_sim::camera::swing` is now that decompose → lerp → rebuild, in R = (`to`'s rotation, pivot). It reuses
  `cutaway`'s `mul`/`rot_y`/`rot_x`/`pitch`/`diff`, now `pub(crate)`.
- `play/change_ends.rs` is unchanged: same start, pivot and end view.
- Test: `hst-sim/tests/camera.rs` `change_ends_swing` replays both recorded glides through `swing`. It uses the
  recorded start, pivot and last shown view, and is skipped without `context/b174/orig/`. Every tick 78..1:

  | Glide | Worst eye error (m) | Worst forward-axis error | Old model's worst eye error (m) |
  |---|---|---|---|
  | Palais | 0.0159 | 5e-4 | 0.24 |
  | Slot 5 | 0.0112 | 4e-4 | 0.26 |

  Most ticks are within 1 mm.
- Slot 5 re-recorded with the pivot field (`research/b174_swing_probe.py 5`, `context/b164/run1/slot5.txt`, copied
  over `context/b174/orig/slot5.txt`). The pivot is (3, 0, −12.25), the server's spot at the −z end, as in the port.
- `change_of_ends_glides` and `change_of_ends_glides_on_the_players_side` still pass (glide 23.5 m, biggest step 0.64 m).

## Not verified / not 1:1

- **End view fixed, not live:** the remaining ≤1.6 cm near the end comes from this. The original's end view is
  its match camera, re-run every tick, and in the recordings it wobbles by a centimetre or two (eye x −0.029,
  −0.014, −0.024 over ticks 7..5). The port computes the match camera's view of the new set-up once. That's invisible,
  and making it live would need the port's camera to wobble the same way.
- **Roll:** the game also takes the look's roll apart and blends it. Both views are upright, so it is 0 and left
  out (`ponytail:` comment).
- **No new `--shot`s:** the change from B174's model is a few cm to 0.26 m at the start of the move. B174's
  tick-40 side-by-side (`context/b174/orig/e40s.png` vs `port/e40s.png`) still stands. This was checked numerically,
  not by eye.
- **Plain f32/musl, not `ps2`:** presentation only, per PLAN.md.
- **Which player 0x76 follows in doubles:** still the first at the camera's end, as B174 left it.
- **The phase's first tick:** the Palais recording still shows the cut-away on the counter's first tick. The port
  starts the glide that tick, as B174 noted.
