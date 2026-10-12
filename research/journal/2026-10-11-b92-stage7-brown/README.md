# B92 — stage 7 brown screen

Blocked on porting the exact rule. The cause is found, and the rule is narrowed to the VU1 clip pass.

## What covers the camera

- It is the structure `w-house2` (plant record 30, index 17, category 19) at (−0.007, −0.003, −25.928), yaw 0.
  It is not the B74 sky/bg follow.
- Its material 0 "wood", packet 0, is a closed box. Local bounds: x ±14.45, y 0…−12.75 (Y-down), z −18.9…−2.4.
- The match camera eye on court 7 is (0, −11.89, −39.238). This was read from the original's camera block, eye at
  +0x10, and it equals the port's eye. So the eye sits inside the box.
- The box's court-side face (local z −2.4) is authored facing inward, so it is front-facing from inside.
- In the port, that face plus the ceiling cover the whole view. That is the "brown sky-like" screen. The texture is
  opaque (tex1, alpha 255).
- Skipping w-house2 in the port (`context/b92/skipw-house2.png`) matches the original's view
  (`context/b92/k07_a.png`).

## What the original does

- **EE visibility.** Per-node sphere-vs-frustum classification, run every frame for each model.
  - Node centre: node +0xc0.
  - Radius: scale × the model's bound at +0x30, reached through node +0x108.
  - Planes, read from the camera block (stored column-major):
    - 4 frustum side planes at +0x5a0;
    - near/far at +0x560;
    - 4 wider "guard" side planes at +0x5e0.
  - Outcomes:
    - outside any plane by more than r → culled (state 0x200);
    - within r of near/far → clip (0x300);
    - within r of a side plane and of a guard plane → clip (0x300);
    - within r of a side plane but inside the guard → 0x100;
    - otherwise 0x100.
- **w-house2 is not culled.** Its node is in state 0x300, byte 0, radius 21.63, centre (−0.007, −3.567, −35.168).
  The model object's visible word is 0x100, and it is linked into the draw list.
  - An earlier note said w-house2's matrix was missing from the DMA buffers. That was wrong as a conclusion: the
    clip path takes a different route.
- **Experiment** (`context/b92/exp_*.png`, slot 8 on copy 3 = court 7 match): poke w-house2's node centre so its
  state becomes 0x100 instead of 0x300.
  - Result: the original draws the exact brown screen the port shows (`exp_z0.png`).
  - With its real state 0x300, nothing of the box shows (`exp_base.png`, `exp_z-10.png`).
  - So the difference is the VU1 pass the 0x300 state turns on: flag bit 0x200 in VU1 word 206.x.
- **VU1** (`context/b92/vu1.dis`):
  - With bit 0x200 set, a reject pass (0x37b/0x385) CLIPs every vertex against a matrix passed in vi1. Any triangle
    with a vertex outside gets ADC + 0x4000 in its w word, so the GS skips it.
  - A clip pass (0x30b) then takes each 0x4000 triangle and transforms it by the matrix at VU 4…7. It trivially
    rejects triangles with all 3 vertices outside one plane (FCOR). Otherwise it runs Sutherland–Hodgman over
    ±x/±y/±z (0x4ad…0x549) and kicks the result.
  - There is no back-face test in either pass.
  - VU1 data memory from the slot-8 save state is in `context/b92/vu1mem_08.bin`.
    - VU 4…7 and 8…11 hold screen-space normalising matrices: a viewport box around 2048±428 / 2048±112, and a
      whole-GS guard of about 100…4000.
    - The z row maps only a narrow Z band near 0xFF0000 inside ±w.
    - So the clip pass's volume isn't the plain view frustum. That would explain why the box's guard-crossing
      triangles end up not drawn, but this was not proven.

## Not verified / not 1:1

- **The exact per-triangle rule in the 0x300 (clip) state.** The matrices the reject pass (vi1) and clip pass
  (VU 4) use at the moment w-house2 is drawn are unknown. VU memory is double-buffered, so the save-state copy is
  only the last packet's. It is also unknown what the clip pass produces for the box's triangles.
  - Unblock: a VU1 interpreter (or the VU1 microprogram in `hst-oracle`) run on w-house2's actual VIF packet, or a
    PCSX2 GS dump of one court-7 frame.
  - A guessed rule such as "drop guard-crossing triangles in clip-state nodes" would also hit the hall floor node
    (state 0x300, r 36), so it can't go in untested.
- **Porting the EE node classification alone** (fully known, above) is not enough. It only decides 0x100 vs 0x300;
  the box vanishing is the VU1 part.
- **Port changes: none.** All debug code was reverted. Only `plan/REFERENCE.md` got the court-index / carousel
  facts.
