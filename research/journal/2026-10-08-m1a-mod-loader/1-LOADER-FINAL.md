# M1a loader

## What loads
- Manifest (`read`): `standard` = 1, every costume file exists, `donor`, `params.base`, `ai_row` in 0–13,
  `hand` right/left (left → hand −1, the disc's Carol/Will mirroring), `params.override` keys must be TParam
  column headers (whitespace-blind, numbers or strings), `face` morph / texture (needs `face.json`) / none.
  Errors read `<path>: <reason>`.
- Costume (`load`): `standard §7` checks ported from `modding/tools/check.py` — root `game_space`, one skin,
  54 core joints with their parents ("X under Y (want Z)"), rest = bind (< 1e-3), core bone axes within 10°,
  Bip01 height (FAIL outside 0.2–2 m), ≤ 4 weights summing to 1 → reject; WARN rules (height 0.4–1.3, feet at
  y 0, face channels, blink) are logged. Joints go in skin order, a non-joint parent is skipped to the nearest
  joint. Morph targets are the union of every part's `extras.targetNames`; disc `.MOR` tracks bind by the name
  after `\x01`. Racket: `racket.glb` beside the costume, else the donor's `PC/PCnnC00.XB` racket.
- Motions, paths, pelvis, arm table, face tracks, stance ball: the donor's, through `disc_motions`.
- Voices: `voice/<program>_<key>.wav` (PCM16, mono or stereo, any rate), grouped by program, sorted by key.

## Test data (local, git-ignored)
- `context/mods/test_pc00`: `python3 modding/tools/rerig.py ~/repos/HST-MODS/out/models/hst/pc00_c00.glb
  context/mods/test_pc00/model/c00.glb` with `mod.json` donor 0, hand left, overrides `Serv POW` 4, `SPE` "10",
  `リーチ(cm)` 170.
- `context/mods/test_broken`: the same glb with node `Bip01LThigh` renamed `LeftThigh` (JSON chunk edit).
- `context/mods/fore_pc00_phoebe`, `getagrip_pc00_emi`: symlinks into `~/repos/HST-MODS/out/mods/`.

## Results
- `tools/check.sh -p hst`: 54 tests pass, including `mods::rerigged_mod_plays_forehand_and_run` (rerigged pc00 vs
  disc pc00: forehand 0x10 and run 3 core joints within 1 cm, arm table, racket, face track count),
  `packaged_mod_plays`, `rejects_non_conforming_mod`, `checks_the_manifest`, `audio::plays_mod_wavs`.
- `HST_MODS=~/repos/HST-MODS/out/mods cargo test -p hst loads_every_mod -- --ignored`: 87 of 87 mods load.
- Viewer screenshots (`context/m1a/side.png`, motion 16 at radius 6): disc pc00 and the rerigged mod look the
  same; Get a Grip Emi renders skinned and textured with the donor's racket.
- `rerig.py` now writes POSITION min/max for rigid meshes too (valid glTF); same fix in HST-MODS (be83b5c).

## Not verified / not 1:1
- Everything: no original to compare against (mods are remaster-only); checked against the disc character only.
- Normals: VU1 approximation `NORMAL = w0·N`, `TANGENT = (1−w0)·N`, exact only for ≤ 2 influences (M1d).
- glTF materials carry no MTL lighting words (shininess/highlight 0) (M1d).
- Extra (non-core) joints only follow their parents; no noise deformers like the disc's hair (M1d).
- Wav voices: linear resample to 48 kHz, no play-speed pitch, no ADSR release on stop, key wraps modulo the
  takes (M1e); a mod with no wavs gets no voice yet, the donor fallback belongs to M1c.
- Texture faces stay neutral (M1b); `face: none` mods never move their face.
- glTF accessor validation is skipped (older rerig output lacks POSITION min/max); external buffers/URIs are
  not supported, only embedded.
- Mods are not in matches yet (play.rs setup uses disc numbers): TParam row, `ai_row`, hand and voice are read
  but unused (M1c).
