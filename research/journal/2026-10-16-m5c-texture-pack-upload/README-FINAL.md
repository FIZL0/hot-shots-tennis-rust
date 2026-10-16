# M5c — upload texture replacement packs

## What changed
- **Category**: `Meta.kind` (B373/M5a) already is the category (`character`/`textures`/…, absent = character, so old
  entries still read). Upload, My uploads and Browse already filter by the kind tab, so a pack lists on all three and
  My uploads' Take down / Remove all work on packs as they do on characters.
- **Size caps** (`modstore.rs`, `modfs.rs`): a pack may be up to 2 GB zipped/unpacked (`MAX_PACK_ZIP`, `MAX_PACK`),
  other mods stay at 1 GB. `Kind::max_zip()` picks the cap for packaging, download, ranged install and the file picker;
  `modfs::install_with` unpacks at the pack cap and then refuses a non-pack over 1 GB. The UI says it:
  `Kind::cap_words()` in the Upload rule box and the Upload info line ("A texture pack may be up to 2 GB (other mods
  1 GB).").
- **Texture count + before/after** (`modstore::package` → `pack_check`): reads the pack's manifest
  (`mods_textures::read`), opens the disc (`Iso::open(iso)`, which also reaches the browser's mounted pack), takes the
  prints of every disc texture the pack names (`texhash::disc_prints`), decodes the replacements in parallel
  (`textures::parallel`, now `pub(crate)`), and stores the count in `Meta.textures` (skipped when absent, dropped for
  non-packs). The store preview of a pack is a PNG with the largest replaced disc texture (nearest-neighbour scaled)
  on the left and the pack's picture (Lanczos) on the right, at most 512 px high.
- **Disc copies refused** (`texhash::is_disc_copy`): a replacement whose pixels equal the disc's texture (alpha-0
  pixels count as 0), or are a whole-factor nearest-neighbour blow-up of it, stops the upload with
  "N of its pictures are the disc's textures unchanged (a pack mustn't ship them): <files>" (up to 5 names, then
  "and N more").
- **Menus** (`mods_screen.rs`, `grid.rs`): the Upload panel shows "N textures - size"; the prompts for packs say they
  are checked against the disc first; Browse/My uploads cards say "N textures" and "A texture pack for this game", and Browse files packs under a
  "Hot Shots Tennis" heading (not "Other games").

## Checks
- `tools/check.sh -p hst-data texhash` (4: `disc_copies`, `prints_from_the_disc` — 5831 disc textures in 1.6 s),
  `-p hst-base modstore` (16, new `texture_packs_go_up`), `-p hst mods_screen` (22): passed.
- `cargo check -p hst --target wasm32-unknown-unknown --features ship`: builds (only older warnings).
- By hand (`context/m5c/run1/`): a 120-PNG 2× court-1 pack packaged in 1.37 s (98 textures, 8.6 MB) with a right
  before/after picture; a pack with two unchanged disc PNGs refused in 0.97 s naming both files. Upload / My uploads
  / Browse shots at 16:9, 4:3 and 9:16 lay out right.

## Not verified / not 1:1
- **No live upload to the store**, from the desktop or the browser: unattended agents don't write to the live HF dataset
  (M17/M19/B250 precedent). The "done when" check (upload from desktop and browser, see it under its category) is a
  `plan/HUMAN.md` item. Packaging, the disc check and the listing were tested locally; the HF request path is the
  one characters already use.
- **Browser build not run end to end**: it compiles for wasm32; the disc check uses the same `Iso::open` the browser
  build loads the game with. In the browser the check runs on the main thread (≈1–2 s for a court pack) and packaging
  holds the files and the zip in memory, so a pack near the 2 GB cap will likely run out of memory in a tab.
- A pack's tile on Upload/My uploads still shows its first letter; the before/after picture only shows once the store
  serves it.
- Not compared with the original: uploading mods is an addition, and the menus are ours (PLAN.md).
- `index.json` (../opentennis-mods `tools/store_index`) copies the meta through as-is, so `textures` should reach the
  listing; not checked against a live index.
