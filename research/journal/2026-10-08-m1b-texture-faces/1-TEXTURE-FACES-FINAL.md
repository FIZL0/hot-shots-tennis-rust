# M1b texture faces

## What changed
- `mods::load` (face `texture`): `face.json`'s channels with a png become morph weights after the model's own
  targets, so the donor's `.MOR` tracks (`face\x01joy_eye` …) drive them by name; `null` channels stay neutral.
  `materials` (glTF material names) start on the neutral png, in both the StandardMaterial (sRGB) and the GS draw
  (raw). One image per file (an emotion's eye and mouth share one).
- `character::TextureFace` on `CharacterData`; `character::texture_faces` (Update, after `animate`, registered in
  `main.rs`) swaps every face draw whose texture is one of the face's to the pick: first strongest channel strictly
  above 0.5 (channels ordered by name), else neutral. Only touches a material when the pick changes.
- Found on the way: PSP mods (Get a Grip, Open Tee 1/2: 62 of 87 costumes) keep the head and face as mesh nodes
  without a skin under `Bip01Head`; M1a dropped them (headless). They now ride the nearest joint at or above the
  node, whole (vertices into game space at bind, weight 1). Standard §2 says so.
- Standard §4b rewritten to what the loader reads: `materials` names the glTF face materials (Get a Grip's
  `<stem>_face0`; the `face`/`faceL` halves are one material in the glTF). HST-MODS 6c781b2 (not pushed):
  same text, `package.py` writes `materials`; the 15 local `out/mods/getagrip_*/face.json` patched the same way.
- Viewer: `spawn_viewer` starts the motion with `Motion::set` so its face plays (was face 0 always);
  `HST_VIEW_YAW` env turns the viewer camera (to see faces).

## Results
- `tools/check.sh`: 294 tests pass; new `character::tests::texture_face_picks_the_strongest_channel`,
  `mods::tests::texture_face_follows_the_donors_faces` (Get a Grip Emi, `context/mods/getagrip_pc00_emi` →
  symlink into `~/repos/HST-MODS/out/mods/`): 9 channels bound, face starts neutral in both material kinds, donor
  13's faces leave neutral on some frames.
- Viewer shots (`context/m1b/sheet.png`, yaw 3.5, radius 3.5): motion 0 neutral, 0x2c joy (zero_face5), 0x2d
  anger (zero_face8), 0x2e joy — through the GS draws (`shade::own_materials` replaces the materials in the viewer
  too). Log during 0x2c: joy weight crosses 0.5 at the 4th frame and the texture switches there.

## Not verified / not 1:1
- No original to compare: texture faces are remaster-only (Get a Grip's own per-motion face swaps in
  `out/anims/getagrip/face_tracks.json` are not used; the donor's HST `.MOR` tracks drive the channels, as §4b says).
- Open Tee 1/2 mods ship `face: none` (their face textures aren't labelled yet, HST-MODS todo), so only Get a Grip
  shows texture faces; Open Tee faces stay on their glTF texture.
- In a match (`--play --mod`) not screenshotted; the same system and GS draws run there as in the viewer.
- Upscaled face textures (`textures/upscaled/`) not looked up: the mod loader has no upscaled-set support yet (M1).
- Rigid head meshes: no lighting check against anything (M1d covers mod lighting).
