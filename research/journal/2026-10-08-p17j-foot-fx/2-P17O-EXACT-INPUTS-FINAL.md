# P17o: footsteps, exact inputs

Closes the input gaps P17j left: see "Ponytails / gaps" in 1-FOOT-TORNADO-FINAL.md.

## Findings
- **State byte +0x3fa5.** The mode setter writes it every frame. The point setup writes 0.
  - Values: 0 stand, 1 run (stick length > 0), 2 stroke/dive/whiff (a press sets 2 at once), 3 body hit.
  - In foot_s05 it does not change once the run object's after-point flag (+0xc0) is set: it is frozen from the
    point's end.
  - The recording also has (motion 4/5/6, state 0) and (motion 48, state 1) frames, so it can't be derived from the
    motion id.
- **Sub-state +0x3db0.** The players' reaction message sets it: 0x2b when the state is 3 (body hit), otherwise the
  reaction id (0x2c–0x2f, team ones +0x30). The point setup clears it.
- **Puff draw.** center = pos + 0 − (camera→world row 2) × 0.5. The rows of the camera→world matrix are right, down,
  forward, eye, so row 2 is `g.cam.view.rot[2]`.
- **Wet flag.** The run object's reset at a point's setup sets:
  - +0xa090 (dusty) = court dusty && !wet
  - +0xa091 (wet) = weather byte (gm+0x84 → +0x135) ∈ {2, 3}, which is `hst_sim::weather::rain`
- **Wind.** Done in the PART iteration: `hst_sim::weather::wind`.

## Port (`hst/src/play/foot_fx.rs`, `bodyhit::posed` made `pub(super)`)
- State: 3 if standing hit by the ball; 2 if contact, pending, wait_swing, swing, whiff or dive; else 1 if
  running; else 0. It is held in `FootFx.states` while `after_point`.
- Sub-state: 0 until the players react; then 0x2b for the hit player, else the root reaction motion.
- Toes, pelvis, spine, head and the player matrix come from this tick's pose: `bodyhit::posed` at `clock.sampled`,
  then `pose::node_world`. They no longer come from the last drawn frame.
- Puff push: `madd(add(pos, 0), row2, -0.5)`.

## Test
- `tools/record_foot.py`: `FOOT_RAIN=<weather>` pokes the weather byte every frame.
- `context/fixtures/foot_s05r.bin`: `FOOT_RAIN=2 record_foot.py 5 3000`, court 10. The flags go from (dusty 1,
  wet 0) to (0, 1) at the first point reset (vsync 8713).
- `footsteps_rain_s05` (shared body with `footsteps_s05`): from the first reset on, the recorded flags equal the
  app's derivation. Steps, puffs and prints are bit-exact.
  - Totals: 1872 wet frames, 341 puffs, 279 footprint frames.
- The harness now accepts a doubled update whose first tick ran on the last frame's inputs.
- One frame (1474) follows a three-frame toe stall. The game caught up through a pose that never shows in the
  recording, so the test resyncs there. Stalls are counted, and fewer than 5 are allowed.

## Not verified / not 1:1
- **App state/sub-state derivation.** It is checked only against the decompile and the recorded pattern; no
  app-level replay compares it with the game frame by frame.
- **Puff billboard axes.** They still come from the Bevy camera (only the push uses the game's view row).
- **0.5 push rounding.** Doing it as one madd instead of the game's mul then sub is argued (×0.5 is exact), not
  measured against a draw capture.
- **`bodyhit::posed`.** It keeps its own ponytail: a crossfade node the base clip doesn't key mixes from the other
  clip, not from the game's last held value.
- **Motion anchor.** Still the player matrix minus the dive direction (a ponytail in `foot_fx.rs`).
- **Missed updates after a pose stall.** Their inputs are not in the recording, so that frame is resynced, not
  checked.
- **Rain wind.** Wind at other angles and speeds was not re-checked with the rain fixture: the slot's wind is 315°,
  speed 2 in both recordings.
