# B49 — change of ends: the original's sequence and the port

## What the original does (slot 5, bot singles, `research/change_ends_probe.py` → `context/b49/run1/`)

- The change-ends phase lasts **81 ticks**. In run1 that was vsync 14154..14234, and the serve set-up (phase 2) followed at 14235.
  The app's `ChangeEnds(80..=0)` is the same length, so match timing is unchanged.
- **There is no walk.** Both players already stand at their new serve placement on the phase's first tick. The
  "animation" is the **camera**:
  - On entry, the camera handler gets the change-ends message. It saves the view on screen at that moment (the post-point
    cut-away close-up), arms an 80-tick counter, and selects a reference frame to pivot about.
  - Each tick it counts down first (79 on the first tick) and blends from the saved view to the match camera's view.
  - The weight is w = (t^((1−t)^2))^2 with t = 1 − left/80. The powf nesting was checked in the disassembly.
  - It breaks each view into orbit parameters about the pivot (heading, pitch, the eye's angles in the heading frame,
    distance, fov), lerps them, and rebuilds the view.
  - At 0 it shows the match camera again.
  - "Change Sides" (P12b3) is up during the glide, and the HUD comes back at the end (screenshots in `run1/`).
- **CPU+CPU:** the match camera object never turns. It stays behind −z for the whole match, so the far end changes
  players instead.
- The app already does this: `cam_owner` is None with no humans, so there is no turn. With more than one human, the
  camera keeps its end by the user's P16b rule. A solo human gets the camera turned to their end (`g.cam.turned`), as before.

## Regression / jerkiness (task parts 1–2)

Before this change, entering ChangeEnds cut instantly: `reset_positions` → `cam.cut()`, `cam_cut = true`. You saw a
jump to the new set-up with nothing in between. "Ran before the players moved" was this cut landing while the post-point
cut-away was still on screen.

The Change Sides pop-up itself was not missing: it still starts on `ChangeEnds(CHANGE_ENDS)`, and app screenshots show it.

## Port

- `hst_sim::camera::{SWING, swing_weight, swing}` does the orbit blend. The easing is exact, using `musl::powf`.
- `crates/hst/src/play/change_ends.rs` is a FixedUpdate system that runs after the replay camera and before the effects:
  - On the phase's first tick it takes the last shown view as `from`. For `to`, it steps a copy of the match camera on
    the new placement.
  - It clears the pending cut and shows `swing(from, to, swing_weight(n−1), pivot)` each tick, then `to` for the last two ticks.
  - It only touches presentation (`g.cam.view`, `prev_view`, `cam_cut`), not the sim RNG or match timing.
- In `play.rs`: `mod change_ends;` and one `add_plugins` line.

## Tests

- `hst-sim/tests/camera.rs` `change_ends_swing` checks:
  - the endpoints;
  - that the mid view keeps the same horizontal distance;
  - the weight: w(79) ≈ 0, w(0) = 1, and monotonic.
- `play/change_ends.rs` `change_of_ends_glides` runs headless bot singles to the first change of ends (tick 2791) and checks:
  - the ends swapped and player 0 is at the new end on the first tick;
  - the phase lasts 81 ticks;
  - each tick starts where the last ended (no cut);
  - no step is bigger than a quarter of the glide (glide 39.48 m, biggest step 1.05 m);
  - the last two ticks show the match camera;
  - the serve continues from it.
- App screenshots (`context/b49/app/t45..t48.png`, `--shot-at` 45/46.2/47/48): the Game/Server pop-ups, then
  "Change Sides" over a mid-glide view with the players already at their ends, then the normal −z match view.

## Not verified / not 1:1

- **Pivot:** I read the original's reference frame (0x76) only later in the match, where it sat near the
  server/ball, not on the phase's first tick. The port pivots on the next server's ground spot, so the arc's shape is by eye.
- **Blend end view:** the original blends toward its per-tick camera object (the mode it switches to). The port uses the match
  camera's cut view of the new set-up, computed once. They should be close, since the players stand still during the phase,
  but this wasn't compared frame by frame.
- **Not bit-exact:** the lerp/rebuild uses f32 + musl rather than `ps2` ops. It is presentation, so by eye per PLAN.md.
- **P1+P2:** not checked against the original, because there is no save state with two humans. The app follows the user's
  P16b rule (the camera keeps its end).
- **Solo human:** the original's swing for a human whose end changes (the camera turning 180°) wasn't recorded. The
  port blends from the cut-away to the turned match view the same way.
