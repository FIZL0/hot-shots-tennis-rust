# P17s — VU1 light for the ball and characters

## What the game does
- VU1's per-draw setup: 109 = light colour × material/128 × the model's light scale (12(vi13).z); 112 = ambient ×
  material/128; lit colour = vc·(109·diffuse + 110·(2nd light) + 112) (+ specular). The light scale only scales the
  directional colour.
- The EE fills the colour matrix with 0x138410(block, params): row0 = RGB × light factor (+0x74), row1 = RGB × third
  (+0x78), row3 = RGB × ambient (+0x70). Glare (flag +0xf0 and DAT_001bb538, −dot > 0.8) dims ambient
  ×(1 − x/2) and light ×(1 − 0.6x), x = (−d − 0.8)/0.2.
- The court light (0x32e5c0 → 138410(0x1e7d10)): envir row RGB, ambient +0xc, light +0x10 × the weather scale
  (rain greys the RGB to its mean).
- The model light: 0x32e5c0 also stores ambient = row +0x18 (or +0xc if 0), light = row +0x1c (or +0x10), third
  +0x20 (or +0x14), NOT × the weather scale; 0x338bf0 (type 0x11) copies the scene's light block (the greyed RGB too)
  to 0x423960 and runs 138410 with them. Slot 5 RAM: light 0x423d80 = (0.5349, 0.52031, 0.52031), ambient 0x423db0
  = (0.59529, 0.57906, 0.57906); same direction as the court's (the sun).

## Checked against the game
- `research/p17s_capture.py` (slot 5, light block read before and after an F11 GS dump: `context/p17s/s5.gs`, `.png`).
- `noidump <mdl> <mtl>` now prints materials (`M`), vertex colours (`V`) and normals (`E … nx ny nz`);
  `research/p17s_light_gs.py dump model.txt` matches the model's packets and prints GS colour / (vc × material).
- Court structures: min ratio 0.62–0.63 → ambient 0.638 (court light). Characters: min 0.600/0.583; ball: min
  0.6016/0.5859, max 1.094 → the model ambient 0.5953/0.579, not the court's.
- `gs::model_light` reproduces the RAM values from envir c10's row (test `model_light_row`).

## Port
- `character::CharacterData::gs`: each part's GsMaterial draws (raw texture, first packet's PRIM, lod_k), by its
  StandardMaterial; the ball's via `shade::BALL_GS` (filled in `main::models`, keyed by texture).
- `shade::own_materials` swaps every new rig's and the ball's StandardMaterials for own GsMaterials (a second TEST
  draw is a sibling with the same mesh/skin/morph; runs after `noise::attach` so swaying parts keep their meshes).
- `shade::light` (after `weather::apply`, which overwrites every GsMaterial on a weather change): ambient + light
  colour × the scale (players 1, rain 1), sun direction, the court's main fog; `DIRECT` is gone.
- Screenshot: `context/shots/p17s/port.png` (court 10) next to `context/p17s/s5.png`.

## Gaps (PLAN)
- P17s1: character max ratio 1.19 > ambient + light 1.13 (ball ≤ 1.094): specular, unnormalised skinned normals or
  else — needs a per-vertex match of a posed character.
- P17s2: the third light (+0x20/+0x14) and the sun glare, for the court and the models.
