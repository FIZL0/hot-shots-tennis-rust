# M5d — download texture packs from the store

## What changed
- **Browse → Texture packs**: B372's rail for packs: Covers (Everything / Courts / Characters / Menus & HUD / Mixed),
  Kind (Upscale / Recolour / New look), Size (Under 100 MB / 100 MB to 1 GB / Over 1 GB), Show. Filters, their
  words, the strip's "unfiltered" and the filter step are kind-aware (`grid.rs`: `PACK_GROUPS`, `Opt::Covers/Look/Size`).
- **Store meta**: `Meta.covers` / `Meta.look` (u8 indices, packs only), filled at upload from optional mod.json
  fields `"covers"` / `"look"` (`mods_textures::tag`: any case, "recolor", "menus & hud", "new look" accepted).
- **Tiles**: a pack's picture (the store's side-by-side before/after PNG) has "disc" / "pack" badges on its halves
  and "N textures" under the name. Installed / Update marks are the characters' ones.
- **Detail**: the before/after wipe: the disc half left of a divider, the pack half right of it, from the same place
  in both; a pointer puts the divider where it is (24 hit strips, `Act::Wipe`), else it sweeps slowly (a pad's ←→
  move between tiles). Facts: "Replaces N textures - Covers - Kind"; not installed: "Installs at the top of your
  texture packs, switched on". The page's line says "Online: only you see a texture pack" (M5c's, = looks only).
- **Install**: the store's install path (B204 streamed progress, desktop `mods/`, OPFS in the browser), then for a
  pack: a new one goes to the top of the pack order and on (`mods_textures::on_top`/`topped`, written to
  settings.txt), an update stays where it was; `textures::repack` re-reads the packs and re-decodes every live
  texture whose pack file changed (or is under the new pack), so it shows at once, no Reload prompt. Status: "Installed X: on, at
  the top of your texture packs. No reload needed." (or the Texture Replacements-is-off hint).
- By-author view and Download all already followed the kind tab (B255/M5b); the test checks it for packs.
- Menu renderer: `Tex::Thumb` now honours a partial `src` (a share 0..1 of the picture); before, it always drew the
  whole picture, so the wipe's halves came out wrong (found in the first shot).
- Sample screen: `HST_MENU=browsemods HST_KIND=3` gives packs Covers/Kind; an offline sample reads the picture
  cache (never fetches), so a planted `mod-pictures/<sha>.png` shows.

## Checks
- `tools/check.sh -p hst mods_screen` (27, incl. new `store_texture_packs`: filters, size bands, wipe hits,
  author view/Download all by kind), `-p hst pointer` (6), `-p hst-base modstore` (16, `texture_packs_go_up`
  covers/look), `-p hst-base textures` (repack applies a fresh pack to live images), `order_and_settings` (on_top).
- clippy clean on the changed files; `cargo check -p hst --target wasm32-unknown-unknown --features ship` OK.
- Shots (context/m5d/run2, run3): 1280×720, 960×720, 720×960 with a planted before/after picture. Tile split and
  badges, detail wipe (blocky disc left, sharp pack right), facts lines; uniform scale at all three; run1 shows the
  bug the Thumb `src` fix cured.

## Not verified / not 1:1
- **Live store**: no upload on one machine and install on another or in the browser: unattended agents don't write
  to the live HF dataset (as M5c). Human check in plan/HUMAN.md.
- **Browser**: compile-checked only; OPFS install goes through the existing store path, not run here.
- **Hot reload in a match**: repack runs in the menu; the match's textures come from the same live images, but
  a texture no pack covers any more (an update dropping a file) keeps the pack picture until restart (the disc
  picture isn't kept to put back).
- **Tile split** is side by side (vertical cut), as the store's preview PNG is, not a diagonal.
- **Divider**: mouse/tap over 24 strips; a finger drag moves the cursor rather than sliding smoothly; a pad sees the
  sweep only (←→ belong to the tiles).
- **Covers/Kind** come from optional mod.json fields: a pack that doesn't say passes only "Any". The standard doc in
  ../opentennis-mods doesn't list `covers`/`look` yet (follow-up task added).
- Menus are ours to design: not compared with the original (an addition).
- Pre-existing flake: `upload_all` once saw a real store listing ("79 mods in the store.") from another test's
  `modstore::list`; passed on rerun, not from M5d.
