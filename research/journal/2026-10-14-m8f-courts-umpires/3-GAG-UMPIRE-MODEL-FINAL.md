# M8f4 — Get a Grip umpire model + motions

## What was done
- open-tennis-mods `tools/psp2gltf/export_getagrip_umpires.py` (commit 30b8165 there): for u_00..u_07 builds the raw glTF
  with `build.py` from `umpire.xb` (`u_NN.psp.i3r`, motions `u_NN_{h,r,l,w}.i3m`, u_00's where a umpire has only
  `_s`), then bakes it into game space under `game_space` (Y down, metres, facing +Z): joint locals K·L·K⁻¹ with
  K = f·diag(−1,−1,1), f = 0.8554/57.9 m per GaG cm (Gloria's calibration, as the GaG player mods), head ×0.6277
  about Bip01Head, inverse binds recomputed, animation keys converted. Animations: `idle` = `_h`, `turn_r` = `_r`,
  `turn_l` = `_l`, `wait` = `_w`. Writes `out/mods/gag_umpNN_<name>/umpire.glb` and drops the model todo from the
  M8f2 `mod.json` (adds `model_why`). `gag_umpires.py` only lists the todo when there is no `umpire.glb`.
- Found on the way: GaG's motions key Bip01Pelvis 45.7 cm lower than the bind pose (the model's origin is its feet
  at bind but the seat in motion). The seat lift is taken from `idle`'s first pelvis key, and ROOTNODE/locators are
  added to the skin: the game's rig keeps only skin joints, so a lift on a non-joint ancestor was lost (first
  in-game shot had her sunk half a metre into the chair).
- Standard §3b confirmed and filled in: the three disc motion slots (idle frame 0 during play, played and looped
  from frame 16 after the match; turns held), optional `wait` looped during play, missing turns hold idle frame 0,
  turn_r = right arm toward +X, head joint `head`/`Bip01 Head`/`Bip01Head` with the model's own axes.
- HST: `crates/hst/src/mods/umpire.rs` — `read` (kind umpire, standard 1, donor 0..4), `load` (skinned + rigid
  parts as a character mod's, clips at 60 Hz → frames; idle required), `--umpire-mod DIR` (main.rs `parse`; sets
  the match's umpire number to the donor, so timing and calls are the donor's). `play/npcs.rs`: the umpire slot
  places the mod rig instead of the disc one (same world transform, same controller in `step`); new systems
  `mod_umpire` (after `step`: `wait` loop during play, missing turn → idle frame 0) and `mod_look` (after `look`:
  head toward `Umpire::head` about the head's y then x, 0.7 of the angle, yaw ±70°, tilt 35° down/25° up). No
  play.rs change; nothing the sim reads (umpire state, RNG, timing) is touched: the rig is draw-only.
- Test: `mods::umpire::tests::a_get_a_grip_umpire_loads` loads all 8 from ../open-tennis-mods/out/mods when built (skipped
  otherwise: game data), checks the 4 clips, lengths, pelvis 0..15 cm above the seat in the idle.

## Verified
- Shots (`context/m8f4/run2`, `run3`; `HST_EYE` at court 10's chair): mod vs disc umpire at the same camera: seated
  on the chair at the disc umpire's height and size. All-bot run (`HST_AUTOPLAY=1`) at 12–32 s: she turns (arm out)
  after the 15–0 point, `wait` breathing, head follows the ball.
- Turn direction: a throwaway probe sampled the disc umpires' clips (all 5): motion 1 (`_r`) ends with the right
  hand at ≈(0.53, −0.40, 0.02) (out toward +X), motion 2 mirrored; baked GaG `turn_r` right hand at
  (0.51, −0.31, −0.01), `turn_l` mirrored. Same side.
- `tools/check.sh` (full) via `tools/done.sh`.

## Not verified / not 1:1
- Which GaG motion the PSP game plays when: its code (EBOOT) is encrypted; mapping is by file name (same `_h/_r/_l`
  as HST's disc umpires) and by rendered poses.
- `_s` (head-turned gesture), `_r_w`/`_l_w` (arm-out loops) and the `.tat` texture animations (blinks) are unused:
  HST has no slot for them.
- u_01–03 and u_05–07 have no motions of their own besides `_s`; they use u_00's (same skeleton).
- Size (Gloria calibration), head ×0.6277 and seat height (52.9 cm under-seat measure on u_00) are chosen to look
  right, not taken from the PSP game (which has no HST-scale counterpart).
- Helghast's canister (`Bip01RThigh9…` attachment) floats beside the chair: GaG's umpire skeleton has no thigh bone,
  so it rides the pelvis.
- The mod head-look uses the original's 0.7 factor and limits on the model's own axes, by eye; the disc umpire's
  look is bit-checked (`b78_head.py`), this one only looks right.
- `wait` replaces the disc's still idle frame 0 during play: an addition (presentation only), off without a mod.
- The umpire select / mod listing and choosing it in the menu are M8f3's; here only `--umpire-mod DIR`. A menu
  umpire choice made after launch doesn't drop the mod model.
- Not compared with the PSP game running (no PSP emulator setup here); compared with HST's disc umpire only.
