# Tutorial: rerigging a character to the HST standard

Read `HST-CHARACTER-STANDARD.md` first. Commands run from the repo root. Never commit game data or anything
exported from it (models, textures, voices); keep it under `context/` or your own folder.

## 0. Inputs

- **Your model**: a skinned `.glb` whose joints use 3ds Max Biped names, with or without spaces (`Bip01 L
  UpperArm` or `Bip01LUpperArm`). That's what lets one tool rerig models from any source.
- **HST's own characters** (donors, reference): export them from your disc:

```
cargo run --release -p hst-data --bin xbdump -- "Hot Shots Tennis (USA).iso" context/xb   # once
cargo run --release -p hst-gltf -- hst context/xb context/models/hst                        # pcNN_c00.glb, all motions
```

Python needs `numpy` and `Pillow`.

## 1. Rerig

```
python3 modding/tools/rerig.py MY.glb context/rerig/my.glb \
    --preview context/models/hst/pc00_c00.glb --motions sh_pc00_f_t,mo_pc00_run_f
```

What it does (`modding/tools/rerig.py`):
1. Converts the source to game space (`G·K`: glTF Y-up → Y-down, source units → metres via the skin's own
   mesh-to-world transform).
2. Maps joints by name (spaces removed). Missing core joints are **synthesized**: on the line to the next mapped
   joint at HST's ratio (e.g. `Spine2` between `Spine1` and `Neck`), else at HST's offset scaled to the model
   (finger segments, `Racket`, `Bip01`). The printout lists them.
3. Builds each core joint's frame as HST's frame swung onto the model's bone direction (single-segment fingers
   use the `…Nub` marker for direction). This is the step that makes HST's absolute local rotations pose the mesh.
4. Keeps every other node as an extra under its nearest kept ancestor; drops scene roots/locators above the core
   (their weights go to `Bip01Pelvis`).
5. Rebinds weights, rewrites IBMs, renames morph targets (drops the `obj\x01` prefix) and adds alias targets for
   missing face channels.
6. `--preview` copies the donor's motions, scaled as the game scales them, for checking.

## 2. Check it — always look

```
python3 modding/tools/check.py context/rerig/my.glb
python3 modding/tools/render.py context/rerig/my.glb context/rerig/my.png --anim sh_pc00_f_t --times 0,0.4 --views front,side
```

Open the PNG. Compare with the donor itself (`render.py context/models/hst/pc00_c00.glb … --anim sh_pc00_f_t`).
Look for: limbs pointing the same way as the donor's, no stretched/torn vertices, feet on the ground, hands
closing on the racket position. Also try `mo_pc00_run_f`, `sh_pc00_serve_t`, `re_pc00_gu` (win), `re_pc00_di`
(loss).

Known failure signs and fixes:
- *Arm twisted along its length*: the source bind roll differs a lot from HST's; check the source's palms
  orientation in bind pose. Swing-only alignment keeps HST's roll.
- *Mesh explodes*: wrong K/units (multiple skins with different mesh spaces) — print `skin_binds` K per skin.
- *Part floating / static*: its weights sit on an extra joint whose ancestor was dropped; check the extras tree.
- *Face doesn't move*: no targets, or names don't match §4 — print `mesh.extras.targetNames`.

## 3. Faces

- Morph targets present → rerig maps them. Check `extras.targetNames` contains the 8 required (§4a).
- Whole-face texture swaps (e.g. PSP games) → write a `face.json` (§4b).

## 4. Package

Make `mods/<id>/` as in the standard: costumes from step 1, `mod.json` (pick `donor` by body type and play
style, `research/characters.md`; map stats into `params.override`), voices renamed to `<program>_<key>.wav`
(table in §5). Upscaled textures, for every mod at once:
`python3 modding/tools/upscale_mods.py MODEL.pth [mods/]` (headless chaiNNer, `tools/chainner/upscale.chn`, skips
what's already upscaled; rerun after adding or rebuilding mods).

## 5. Report

Per character: mapped/synthesized joints (from the rerig printout), face channels present/aliased, the render
PNGs you checked, and anything odd.
