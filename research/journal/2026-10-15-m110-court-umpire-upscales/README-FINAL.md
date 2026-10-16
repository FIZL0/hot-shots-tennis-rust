# M110 — chaiNNer upscales for the Get a Grip court and umpire mods

## Before
`tools/upscale_mods.py` only read character mods (costume `.glb`s + `racket.glb`), and the game only looked for
`textures/upscaled/` there (`mods.rs` `materials_of`). Court mods (`court_mod.rs` has its own `materials`) and
umpire mods (`mods/umpire.rs` passed `None`) never read upscales.

## Changes
- `upscale_mods.py`: also reads court mods (`court`/`gimmick`/`sky.glb` + the `npcs.json` people models) and
  umpire mods (`umpire.glb`). Same `textures/upscaled/<glb stem>/<image>.png` layout. Each mod folder is now visited
  once by its resolved path: `out/mods/mods` is a symlink to `out/mods` itself (made 2026-10-15 09:11, not by this
  task, left in place), and the recursive glob went ~40 levels deep through it, so every mod was queued ~40 times.
- Game: `mods::upscaled_in(dir, glb)` / `mods::upscaled_image` (factored out of the character path). Court
  `Store` has a new `upscaled` dir, set for each court/gimmick/sky glb (`court_mod.rs`) and each person
  (`court_people.rs`). The umpire's glb passes its dir. The setting and browser rules are the same as for characters
  (`textures::on()`).
- README: the upscaler covers character, court and umpire mods.

## Run
Model `~/.local/bin/chainner-app/HDcube4/Models/HDcube/4xHDcube4.pth`, the one the character upscales were made
with (shell history). All 16 `gag_c*` courts (2121 images) and 8 `gag_ump*` umpires (26) are done in
`../opentennis-mods/out/mods/*/textures/upscaled/`. The first pass reported 2121/2121 but left 11 (court shadow /
baked maps) unwritten. A rerun did them, and a final `todo()` over `gag_*` is empty. Courts took ~10 min,
umpires 27 s.

## Verified
`context/m110/ab.sh COURT UMPIRE`: `--play --shot-at 4` with `HST_WEATHER_SEED=1234`, from a disc-only ISO folder,
once with upscaling on and once with `--no-upscale`. A temporary eprintln (removed) logged every image lookup.
Every court with its umpire (umpires 0–7 paired with courts in turn, each umpire twice): **0 misses**. Every umpire's
images (3–4) were found. Between on and off, 3.6 % (c17, mostly flat colour) to 28 % of pixels changed by more than 8.
Visual checks: c01 grass and Natasha's face and shoes are visibly sharper; c17 looks unchanged apart from fine detail.
Summary is in `context/m110/ab/`. A shot without `--play` is the disc court viewer, which never loads the court mod.

## Not verified / not 1:1
- Rendering only (no gameplay touched), so there is nothing to compare bit for bit against the original. The A/B
  shots only compare our own renderer with and without the upscales.
- `lensflare.png` and `previews/*.png` (loose PNGs, not in a glb) are not upscaled: the flare is a small sprite and
  the previews are menu thumbnails.
- Why the first pass skipped 11 images while counting them as written is unexplained. The rerun fixed them, and
  `upscale_mods.py` reruns are idempotent.
- The task covers only the Get a Grip mods: the 3-image racket changes in other character mods and the
  `hst_umpire_*` mods that a full run would now also pick up were not run.
