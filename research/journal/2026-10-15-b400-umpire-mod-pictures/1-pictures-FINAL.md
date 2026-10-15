# 1 — pictures from previews/umpire.png (FINAL)

## Where they were missing
The Mods screen doesn't list umpire mods yet: Installed holds characters only, and the Umpires kind tab shows the
"arrive with M8f" empty state until M8f9. So the only place umpire mods appear is the umpire select on the
confirm screen (M8f3). There a mod got a plain named card on the strip and its donor's disc still on the stage,
and the confirm card used the donor's round face. That select is what this task fixes. M8f9 got a note to reuse
this when the Browse/Installed tiles come.

## What changed
- `play/main_menu/umpire_mods.rs`: reads each umpire mod's `previews/umpire.png` (mods-repo standard §3,
  384×480 RGBA, optional) into `Tex::Umpire(i)`. `head()` finds the head's square (0.36 × figure height, centred
  on the top fifth). Strip card = the head cut to the card's shape plus the name at the foot (as Random has).
  Stage = the whole picture, fitted to keep its aspect. Confirm card = the head clipped round with the disc
  faces' rims. A mod without a picture looks the same as before.
- `play/main_menu.rs`: the `Tex::Umpire` arm in `draw`, the `Pics` resource and `load` system, plus three hooks
  (strip, stage, confirm face).
  `draw` takes `Pics` as optional, because the pause overlay runs `draw` without the main menu plugin (a
  required `Res` made `pause::open_change_back_in_a_match` panic).
- Test `umpire_mod_pictures_find_the_head`: head square from a synthetic PNG; a blank picture gives none; only
  mod values with a picture draw one.
- opentennis-mods `b4d05f9`: `tools/psp2gltf/export_getagrip_umpire_previews.py` writes the eight GaG pictures. Each is
  cut from the umpire's art on their English loading panel (`loadmes_jNN.xb0`). The band is what ≥4 of the 8
  panels share. The name is the part that differs between the English, French and Spanish panels. In the name
  strip only opaque pixels are kept. Then speck removal and hole fill. Documented in README and notes/getagrip.md.

## Checked
`tools/shot.sh` screenshots (in `context/b400/run1/`): `umpires-mod` at 1280×720 and 700×900 (strip cards and
stage keep their aspect, nothing stretched) and `confirm-umpire-mod` (Natasha's round face). The `hst` tests pass.

## Rejected
Rendering each mod's `umpire.glb` (opentennis-mods render.py) as the picture: Natasha, Nate and Helghast are fine, but
u_01–03 and u_05–07 have stray hands/feet, even at rest (an export bug, now task M93).

## Not verified / not 1:1
- No original to match: the umpire select is front-end (ours to design), and HST has no umpire mods. Card and
  stage layout are our design.
- "Mods screen grid" read as the umpire select's strip, because the Mods screen doesn't list umpire mods before
  M8f9 (noted there). previews.rs (the 3D character baker) is untouched: umpire pictures come from the mod.
- The pictures are GaG's own 2D panel art, cut out automatically and scaled up about 2× (soft). Kanani's chair
  edge keeps a faint halo from the panel's band.
- The face crop is a heuristic (top fifth of the opaque figure). Checked by eye on all eight GaG strip cards.
  Pictures from other mods may centre the head less well.
