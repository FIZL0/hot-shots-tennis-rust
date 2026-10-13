# B222 — --original skips upscaled textures

`menu_art::OFF` (disc sheets, wall_k = 1) was only ever set to true, and only by the `--original` flag. Settings.txt's
`original = on` reached `Graphics` but never `OFF`, and `textures::init` (the PCSX2-/key-named pack) ran before
settings.txt was read. Two broken cases:
- a run started with settings' original on: the pack and our rebuilt art were loaded anyway;
- an `--original` match from the menu left `OFF` on for the rest of the run, so the next normal match loaded the
  pack (`textures::set`) and the menu after it tiled the pack's 4× wall with stretch / 1, i.e. 4× too big.

Fix (`main.rs` only): `parse` stores `OFF = gfx.original` on every parse (so it resets too). `start` stores it again
after reading settings.txt, and calls `textures::init` only when `upscale && !gfx.original`. Match runs already
force `upscale = false` under original, so `textures::set` drops the pack and mods' `textures/upscaled/` follow
`textures::on()`.

Checked: `tools/shot.sh` main menu in a scratch folder (ISO + mods symlinks, settings.txt `original = on`):
before the fix it showed the pack wall and our art. After it, the disc's sheets and 1× wall tile (same grid pitch as
the remastered wall). `--original` on the command line: 1× wall, as before. Shots in `context/b222/`.

## Not verified / not 1:1
- The menu → `--original` match → normal match → menu sequence wasn't driven end to end (no scripted menu
  navigation in a `--shot` run). It's covered by `OFF` now being set on every parse.
- Settings' `original = on` at startup still leaves the menu widescreen (`WIDE` only follows the `--original` flag).
  That's outside this task's scope (textures only).
