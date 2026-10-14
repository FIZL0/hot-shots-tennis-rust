# B335 — store previews instead of baking, Mods page fixes

User report: on a phone, "a problem repeatedly occurred" after downloading 40 mods; asked to use the included
preview and lower-res ones for mobile/web. Also asked for: Remove all on Installed, ← on an uploader in Browse to
fold it, Download all showing x/N, and Play with removed (it did nothing).

## Why the phone baked every mod
- `previews::mod_png` checked a mod's shipped `previews/<costume>.png` with `Path::is_file`, and `read`/`cached`
  on wasm only looked in the card cache. In the browser, mod files are in `modfs` MEM, so a shipped preview was
  never seen. Fixed: both go through `modfs`.
- None of the store's mods ship `previews/`. The store does keep `mods/<id>/preview.png`: the uploader's baked
  card, 384×480 RGBA, the same drawing as our bake. So the baker no longer loads and draws each store mod's
  costume 0 in the background. It fetches that PNG once per run (`modstore::installed_preview`, using the rev from
  `store.json`), resizes it to this device's size (`sizes()`, 1× on a phone), cuts the face close-up with
  `inspect::face_square(None)`, and writes both to the card cache under the bake key (`from_store`). If the fetch
  fails, the mod is baked as before.
- Checked natively: with a `store.json` added by hand to `fore_pc13_allan`, the select's costume-0 cache file
  matches the store's `preview.png` pixel for pixel; costumes 1–7 still baked (native only; the browser only bakes
  hovered ones). The face crop of a real store card looks right.
- Not done: the browser still holds every installed mod's files in memory (modfs MEM, ≈10–40 MB per mod). 40 mods
  could still go over an iPhone tab's limit even without baking. If the crash persists, that's the next step
  (read files on demand). Needs an iPhone check.

## Mods page
- Installed: Add mod / Reload / Remove all. Remove all confirms, then deletes every mod the game installed
  (desktop: not folders added by hand). A mod's press opens Remove only. A mod added by hand just says where to
  delete it.
- Play with removed (its `View::play` hook in `main_menu.rs` too).
- Browse: ← on an uploader's heading or one of their mods folds their mods under the heading ("+ BY …"); →
  shows them again. The legend reads Fold / Show.
- Download all: each install's status starts with "done/total done." (`View::batch`, reset when the queue
  empties), including the progress and the browser's save messages.

Shots: `context/b335/` (addmods, addmods_narrow, browsemods with someone folded, browsechild with 1/3).
