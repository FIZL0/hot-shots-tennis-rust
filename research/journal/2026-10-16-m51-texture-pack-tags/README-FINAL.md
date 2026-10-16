# M51 — texture packs' `covers` / `look` in the mods standard and pack tool

Follow-up of M5d (`../2026-10-16-m5d-texture-pack-download/`), whose Browse filters read two optional mod.json fields.

Done in ../opentennis-mods (commit 73f5891 on its main, **not pushed**: the parallel run forbids pushing):
- `standard/HST-TEXTURE-PACK-STANDARD.md` §2: `covers` (everything/courts/characters/menus/mixed) and `look`
  (upscale/recolour/new), any case, the aliases `mods_textures::tag` takes (`recolor`, `menus & hud`, `new look`),
  missing/unknown = only under *Any*, never refused, no effect in play. §4: the Installed tab (M5b) edits the order
  now. New §5: making one with the tool.
- `tools/texpack.py` (new; the repo had no pack tool): EDITED/ of `--dump-textures`-named PNGs → `<out>/textures/`
  + `mod.json` (+ `--zip`, folder inside). With `--src` (the dump) it drops byte-identical files and guesses
  `covers` from the dump folders the keys come from (COURT/COURTSET courts, PC characters, MENU/AZUMA INPANE+PRIZE/
  SAITOU menus; C_EFF, CMN ball, PCDATA = other → mixed; all three → everything) and `look` = upscale when every
  file is a whole ≥2× multiple of its original; otherwise look is left out unless given. Either can be forced or `none`.
- `standard/TUTORIAL.md`: a *Texture packs* section (dump, edit, texpack, install); README's standard summary names
  the tool.
- Here: `mods_textures.rs` COURTS/LOOKS doc points at the standard §2 and texpack.py (keep the three lists equal).

Checked: texpack on scratch copies of the real dump — a 2× upscale of two COURT/01 textures + one unchanged file →
2 textures, `covers: courts`, `look: upscale`, zip layout `<id>/mod.json`, `<id>/textures/…`; a flat COURT+PC+MENU
edit with `--src` → `everything`; identical copies are refused ("no changed key-named PNGs").

## Not verified / not 1:1
- Packs from texpack.py weren't installed in the running remaster in this task (the format is the one
  `mods_textures::read`/`tag` already test; the tool only writes those fields).
- The folder → covers mapping is our reading of the dump folders' contents (AZUMA/SAITOU are developer folders: HUD
  panels and result screens counted as menus, effects not); it is a guess for a store tag, not game behaviour.
- ../opentennis-mods commit not pushed (run rules); push it with the next mods-repo push.
