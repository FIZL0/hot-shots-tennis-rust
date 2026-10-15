# M93 — fix (FINAL)

## Cause
`u_NN.psp.i3r`'s skinned groups (0x4C) carry a flag byte after the palette count: 0x04 for the usual weighted
groups, 0x0C for one-bone groups with no vertex weights. The 0x0C groups' vertices are in that bone's own space
(Kanani's hands span x 0..21 cm from the wrist; her legs -48..-3 cm below the pelvis), but i3d/build.py treated
them as model space like the weighted ones, so they sat around the origin. The inverse binds in their 0x46 skin
are consistent with the skeleton (IB·W = F), so skin_space's IB check can't tell them apart: only the flag can.

Which umpires have them: u_02 (both hands, legs on Bip01Pelvis), u_03 (hands), u_04 (a piece on the pelvis),
u_05 (left hand), u_06 (right hand), u_07 (hands). u_00 and u_01 have none, so Natasha and Nate were fine.

## Fix (opentennis-mods c79756e)
- `tools/psp2gltf/i3d.py`: `MeshPart.bone_local` = flag bit 3.
- `tools/psp2gltf/build.py` `skin_space`: a bone-local group goes to model space by its bone's bind world
  (`world[bone] @ F`), then skins to that bone as before.
- `export_getagrip_umpires.py` docstring notes it; all eight `umpire.glb` re-exported.

## Verified
- render.py of all eight (bind, `idle`, `turn_r` at 0.5 s): every hand on its wrist, Kanani's legs under her skirt.
- In a match: `HST_AUTOPLAY=1 tools/shot.sh … --play --stage 10 --umpire-mod …/gag_ump02_kanani --pads -,-,-,-
  --shot-at 14` (and robo_t) → `context/m93/run1/`. Both sit whole on the chair.
- Elsewhere in Get a Grip, 11 of 3805 i3r files have such groups (bodies 32, 111, 152, 157, 185, 186, 206; three
  court gimmicks). The fix changes their exports too; re-exporting and checking them is task M94.

## Not verified / not 1:1
- Robo T.'s legs stick out forward from the chair: that is the model as authored (no leg bones, legs ride the
  pelvis; same in the bind pose). The PSP game can't be run here, so there's no in-game picture to compare against.
- How Get a Grip itself draws the 0x0C groups is inferred from the vertex ranges and the result: the PSP code is
  encrypted.
- No Rust changed, so the remaster's code didn't need a separate check: `mods_umpire.rs` loads the new glb as before.
