# B259 — an updated mod's select card is drawn again

## Change
- `previews.rs stamp()`: the PNG stem `<folder>-<hash>` now hashes every file in the mod's folder (name, size,
  modified time on disk), plus `mod.json` and `store.json` whole (a store install writes the zip's sha256 there, so a
  store update always changes it, also in the browser where files have no times). It replaces the newest mtime of
  mod.json/motions/costumes, which missed texture-only changes and zip extracts that keep mtimes.
- `start_baker`: computes every mod's stamp once and deletes each `m-…` cache file (PNG and `-face`) that no current
  stamp made: an updated mod's old drawings and a removed mod's. Natively that's `cache/previews/<boot>-r<RENDER>/`, in
  the browser the OPFS `previews/` cards (`web::cards_drop`, through the page's `hstCardDrop`). The baker's queue then
  finds no PNG at the new stamp and bakes it as usual (costume 0 first, hovered first).
- In session (Mods page, B229): a mod installed since startup isn't stamp-memoised, the baker's `extra` copy is
  replaced by the refreshed one, and an install/import clears the page's `local/` pictures, so a store update on the
  Mods page shows the new drawing without a restart.
- Test: `previews::tests::updated_mod_gets_a_new_stamp` (in-memory mod: file content/size, store.json, an added file;
  `stale` keeps only current stamps and the disc's `dNN` files).

## Native check
In the scratchpad: a copy of `opentee2_pc16_lauryn` in `mods/lauryn`, the disc cards copied in, then
`HST_MENU=chars HST_BAKE=all hst disc.iso`, five runs:
1. first bake: `m-lauryn-50e0…-0.png` (+ face).
2. no change: same stamp, nothing re-baked.
3. upscaled textures negated with old mtimes kept: new stamp, old files gone, baked again. The picture is the same
   because the bake doesn't draw the upscaled set.
4. `mod.json` hand left→right with the old mtime kept (the old stamp would have missed it): new stamp, old files
   gone, and the card is now mirrored (before/after compared by eye).
5. mod folder removed: all `m-` files gone, the 280 disc cards kept.

## Not verified / not 1:1
- Browser: not run live. That would need a wasm release build, an ISO import and a mod installed and then updated in
  OPFS through headless Chromium, which is more than this task needs. It uses the same `stamp`/`stale` (unit-tested),
  `modfs` in-memory sizes, and `web::cards_drop`, which is `cards_keep` with a predicate (`hstCardDrop` already
  deletes OPFS cards).
- A browser zip import (no store.json, no file times) that changes a file to another of the same size, without
  touching mod.json, keeps its old card (`ponytail:` note on `stamp`).
- Not 1:1: front-end only, no original behaviour to match.
