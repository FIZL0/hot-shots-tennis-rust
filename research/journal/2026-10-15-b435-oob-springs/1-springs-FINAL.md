# B435 iteration 1 — FINAL

Task: OOB mods' spring bones (ponytails, earrings, ears, cloth strands) still move oddly in a match.

## What was off

1. **Records shifted by one bone.** The old `spring.rs` read each dynamics record as "the mass sits at this bone,
   its parent turns". That dropped the first record of every chain (its parent is the head) and left each chain's
   last link rigid (e.g. `pc00_pony05`, `L_ering03`). It also gave the first earring link (`R_ering00`, limit 50/60,
   not a 0° anchor) its child's parameters. Chains almost always end in a leaf with no record (`R_ering02`,
   `L_ering04`, `m0_pony07`…), so each record is the bone that swings, and its tip is its child.
   Now: a record with a parent and limit > 0 swings about its own origin. Its tip is its spring child, else its
   first child, else its rest offset. A 0° record stays rigid.
2. **Stiffness read as an acceleration.** `v += (target − x)·k` means only damping decides how fast a chain
   settles (|λ| = √0.83). With that, a stiffness-1 anchor rang with a period of about 6 frames, and stiff links
   bounced limit to limit for 40+ frames. OOB's values (k = 1 on 0–5° roots falling to 0.1 at tips) read as the
   share of the way back to the pose taken each step. Each 60 Hz step now keeps velocity through damping, adds
   gravity, then moves `stiffness` of the way back to the pose, then clamps to the limit.
3. **Frame rate.** A match draws at roughly 370–470 fps. The old code held whole 60 Hz steps, so the hair moved in
   visible jumps. dt-scaled fractional steps don't match 60 Hz (peak 30° against 37°), so physics stays on whole
   60 Hz steps. The drawn pose blends between the last two steps' directions, held in the parent's frame.
4. **Coupling.** At first the blended directions also turned the parents that the children's physics hung off,
   which made links flip on alternate frames. The physics pass now poses the spring joints from physics
   directions only, deepest last; a separate draw pass applies the blended directions.
5. **Own motions key hair bones.** The mods' `motions.glb` reactions (`re_gu`, `re_di`, `*_set`) key the earring
   and ponytail bones with 60°+ jumps (`R_ering00` spans 65°). Those tracks were fighting the springs. `own_motions`
   now skips spring joints and everything under them (`spring::moved`).
6. Checked and fine: `character::animate` writes every joint every frame, so swing never builds on itself.
   Mirrored roots (scale x −1) turn correctly (B410 test still passes).

## Measurements (context/b435/)

- Unit whip on Jasmine: before, every link bounced between its limits; after, roots and earrings settle in about
  20 frames.
- Real match (`--play --singles --stage 1 --pads -,- --mod mods/hst-mods/oob_pc00_jasmine`,
  `HST_WEATHER_SEED=1234`, logs `context/b435/run3/{old,new}.log`). Mean frame-to-frame jerk (|second difference|)
  on the earrings fell from 1.3–2.3° to 0.55–0.72°, on `pc00_pony01` from 1.58° to 0.28°, and its 99th percentile
  from 20–31° to 6–8°.
- Stills `context/b435/view/{old,new,pair}.png`: the old ponytail splays into strands with one sticking up; the new
  one hangs together.
- All 17 OOB mods play a match without panicking (`context/b435/smoke/`).
- Stress probe on 11 OOB rigs (removed; it lived in mods.rs as a temporary test): 5 s of running, sway, bounce and a
  90° turn under a mirrored root, then still.
  - Worst mean jerk under forcing: Jasmine 23.7°, Sophie 17.4°, Sasha 9.0°, Fernando 19.0°, Sonia 18.4°, Kate
    0.3°, Dino 2.0°, Anya 10.0°, Suzuki 1.4°, Alex 3.3°, Gloria 0.9°.
  - These are smooth, forced oscillations of light tip links (2–3 cm, k 0.1). The probe's bounce is a sharp 4 Hz
    |sin|, harsher than a real run step.
  - After the forcing stops, every rig moves less than 0.001° per frame within 100 frames, so nothing is unstable.
  - An earlier 206° figure for Suzuki was the probe's own error (`to_scaled_axis` on q with w < 0).
- `mods::tests::oob_hair_swings_and_settles` now checks:
  - 21 swinging bones.
  - No own-motion track on `R_ering00`.
  - The hair trails a whip by more than 5°, settles under 0.5°, and leaves the head untouched.
  - `R_ering00` and `pc00_pony05` (rigid before) swing.
  - 144 Hz matches 60 Hz (peak within 3°, rest within 0.5°), and every 144 Hz frame moves.
- Spring unit tests in `spring.rs` updated to the new semantics and passing.
- Full `tools/check.sh -p hst`: 452 passed, 1 failed. The failure is
  `play::court_mod_match::a_court_mod_is_seen_and_heard_and_never_moves_the_match`
  ("0 of the mod's 0 people placed"). The shared `mods/hst-mods/gag_c01_yamate` was rebuilt at 23:35 by another
  session without its `npcs.json`. That is not related to springs.

## Not verified / not 1:1

- **Out of Bounds' own spring solver isn't ported or compared.** We have no PS3 reference run and no
  side-by-side with Out of Bounds. "Moves like OOB" is judged by eye and by the stability numbers above.
- **What the parameters mean is inferred from their values, not from OOB code.** That covers: record = the
  swinging bone, stiffness = blend toward the pose per 60 Hz step, damping = velocity kept per step, gravity in
  m/s² at 60 Hz, limit = cone in degrees. Words 4, 5, 6, 8, 10 and 11 of the record are still unused.
- **Collision shapes aren't simulated**, so hair can pass through the head or shoulders at the limits.
- **Cloth meshes (skirts etc.) aren't decoded as springs** and stay rigid with their bones.
- **Pre-mirrored left-handed costumes mix parameters** (opentennis-mods `tools/rerig/mirror.py`). It moves each
  joint's mesh to its twin but leaves the spring extras in place, so one drawn earring can use some left-side and
  some right-side records. Fixing it means swapping the extras between twins in mirror.py and rebuilding the
  shared `out/mods`. That is in the mods repo, which another session was editing with uncommitted changes, so I
  didn't touch it. The two sides' parameters are close, so the visible effect is small.
- **Gravity and tip lengths aren't rescaled** by the rerig/M1l size scale. Gravity is applied in world metres.
- **No `--play` video clip was recorded.** Evidence is the per-frame joint logs and stills above.
- `../opentennis-mods notes/oob.md` / standard §3b weren't updated with these readings, for the same reason as
  the mirror.py item. The readings are recorded here instead.
