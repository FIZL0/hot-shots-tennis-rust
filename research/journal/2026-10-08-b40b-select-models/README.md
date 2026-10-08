# B40b — 3D models in the character select (inspect pose)

## What was built

- `character.rs`: `disc_motions` also loads the menu inspect pose from `MENU/PC/PCnn.XB` (`pcNN_pose.ANI` as a Clip, `pcNN_pose.MOR` as a face) under the key `character::POSE` (0x40). `pose_frame(n, length)` gives the frame the original holds. Mods call `disc_motions` with their donor, so they get the donor's pose with no mods.rs change. Test `inspect_poses_bind_and_hold_the_games_frames`.
- `inspect.rs` (new): `inspect::spawn(commands, data, n, hand, layer, rect)` puts one character, posed, lit and framed like the original's inspect screen, into a screen rect on its own camera and render layer. `inspect::apply` plays R2 gu_set (motion 0x2e) / L2 di_set (0x2f) / △ back to the pose. The `--inspect all|N[:C],…|MODDIR,…` viewer shows a row of them. This is what B40's select will call per slot.
- `shade.rs`: a `FixedLight` component lights a rig with a fixed light (no court/weather).

## Findings in the original (PCSX2)

- The pose is held static: the motion clock never advances on the inspect screen. Start frame (the screen's pose-start function at 0x3236b0): if a per-character flag is set (flags by char 0..15 = 1,1,0,1,0,1,0,1,0,0,1,1,0,1,0,0), frame = (char 0 ? 70 : int(anim length)) − 1, else 0. The object's byte +0x7cf holds the character; anim length at +0x2c equals the port's `Clip.length`. Live held frames: c0 69, c1 61, c2 0, c3 62, c4 0, c5 66, c6 0, c7 57, c12 0, c13 64. The inspect grid is not in character order (0,1,2,5,13,4,3,6,7,12,…).
- Buttons: R2 = gu_set (0x2e), L2 = di_set (0x2f), each at speed 1 from frame 0 to its end, then held. △ = back to the held pose at yaw π. L1/R1 change costume. The yaw is set instantly; the loss (L2) yaw is 2.443461 for char 4, 2.356194 for char 9, else π. Sign checked from the world matrix (row0 = cos θ, 0, −sin θ).
- Light (RAM light matrices): two lights, colour 0.4 each, directions (0.7071,0,0.7071) and (−0.819152,0,0.573576), ambient 0.47. A third light (0.383,0,0.924) has zero colour.
- Camera: focal 1194.256 px horizontal (30° over 640) and 1114.64 vertical (448-line frame). The 2.79904 seen in RAM is PCSX2's widescreen patch (×0.75). Axis at pixel (520,176) of 640×448. Model at game (0,1.5,8), yaw π, mirrored in x for left-handers (TParam hand). Near 0.01, far 3072.
- Bevy pitfall: a second camera on the same target is alpha-blended by the upscaling pass, and gs.wgsl writes specular into alpha, so the model looked translucent. Fixed with `CameraOutputMode::Write { blend_state: Some(BlendState::REPLACE), .. }`.

## Verification

`--inspect N` shots of all 14 characters side by side with the original's inspect screen (tools/screenshot.sh); images in context/b40b/side0.png and side1.png (git-ignored). Every pose, facing, racket hand and framing matches; lighting is close by eye. Mods: `--inspect` with three Fore! mods (phoebe/pc00, brad/pc05, falcon/pc09 from ../HST-MODS/out/mods) shows each in its donor's pose (context/b40b/mods.png).

User note (2026-10-08): the in-game Data menu's Costumes tab shows each character's costumes separately, and the Umpires tab has the umpire 3D models — sources for costume previews and umpire-select previews (B40b4).

## Not verified / not 1:1

- The select itself: B40's character select (play/main_menu.rs) doesn't exist yet, so the done criterion (a `--shot` of the select with every slot hovered) can't be met. The previews run in the `--inspect` viewer only → B40b1.
- UVA eye UV animation (`pcNN_pose.UVA`) isn't ported → B40b2.
- Chars 5 and 13 draw gu/di through another path in the original (object +0x840 = 1); not ported. The gu_set root path isn't applied, and the crossfade (if any) on button presses isn't verified → B40b3.
- The zero-colour third light is ignored (no visible effect).
- Brightness/colour: matched by eye on the side-by-sides only, not pixel-compared.
- Costumes other than 0: not compared against the original's Costumes tab; umpire previews not done → B40b4.
- Mods: donor pose on a rerigged skeleton checked by eye for 3 Fore! mods only; their face channels doki_eye/doki_mouth are missing (mod side warning).

# B40b3 — inspect gu/di details

## Findings in the original (menu overlay + PCSX2 copy 4, Data → Items → Characters, scratch slot 9)

- The inspect object's draw (0x3230e0) takes one of two paths: +0x840 = 0 → the plain draw (0x37d160: the model's matrix from its spot, yaw and hand mirror); +0x840 = 1 → 0x37d280: the same matrix, the skeleton posed with it, `Bip01Pelvis` (string at 0x3888f8) looked up, and the matrix moved by (spot − pelvis world) in x and z (y 0) before drawing. So the pelvis sits over the spot (0, 8).
- +0x840 is set to 1 by the R2 and L2 handlers (0x3476e0) when the character is 5 or 13, else 0. △ (0x1000) doesn't touch it, so after a reaction the pose is pelvis-centred too. A character or costume change (0x34ce10) and the screen's init (0x322f70) clear it.
- No root path: the menu loads only `re_pcNN_gu_set/di_set.ANI2` (+ .MOR/.UVA) from `MENU/PC/PCnn.XB` (byte-identical to PCANI's), no `_dummy` path. Live for char 0: the root matrix stays at (0, 1.5, 8) through gu_set (context/b40b3/c0_r2.tsv). The B40b gap note "apply gu_set's root path" was wrong; nothing to apply.
- No crossfade: both switches call the motion setter 0x140360 with a null blend source, and its blend copy (0x140a70) returns at once without one. It's a cut, as the port already does.
- Live for char 5 (context/b40b3/c5_r2.tsv, `research/b40b3_probe.py`): with +0x840 = 1 the pelvis world x/z stays (0, 8.0) and the root z moves 7.79–8.03 over gu_set; after △ the root is (−0.000284, 8.001564). The motion clock reads one frame ahead of the drawn joints (it ticks after the draw).

## Built

- `inspect.rs`: `Preview::centre` (set by `apply` for Win/Loss on chars 5/13, kept through Pose; a respawned preview starts false), `pelvis_centre(skeleton, clip, t, yaw, hand)` (the pelvis's world point through `hst_sim::pose::node_world`, model x/z = 2·spot − pelvis) and a `centre` system after `character::animate` that moves the rig for the drawn time. `apply` now takes `&mut Preview`.
- Test `inspect::tests::pelvis_centre_matches_the_game`: Brad's pose (66) and gu_set frames 0, 12, 15, 59, 120 against the recorded root x/z, within 1e-5.

## Not verified / not 1:1

- Float exactness: `pelvis_centre` uses Rust `sin_cos` for the yaw and plain f32 for the shift, not the game's sin and `hst_sim::ps2` ops. It matches to ~1e-6 on the recorded frames, not bit-exact (drawing only).
- Char 13 and L2 (di_set) weren't recorded live. Same handler branch and draw path per the decompile.
- Frame phase: the original draws clock − 1. The port draws its own interpolated clock (general motion timing, not this task's).
- No side-by-side screenshot: `--shot` can't press R2, so the centring is checked by the test only.
