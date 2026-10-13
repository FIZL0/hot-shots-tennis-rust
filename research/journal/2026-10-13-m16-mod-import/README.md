# M16: add and remove mods in the browser

## What landed
- `crates/hst/src/modfs.rs`: the mod file layer. Natively it reads the disk; on wasm it reads an in-memory map that the
  page fills from OPFS before `run`. wasm can't read OPFS synchronously, so `mods::list`/`read`/`load` and `audio`'s
  wavs go through it.
- Zip import (`modfs::install`). It unpacks with miniz_oxide (stored and deflate) and enforces:
  - plain paths: no `..`, absolute paths, `\`, drive letters, device names (CON, NUL…), symlinks or duplicates
  - the extension whitelist (.json .glb .png .webp .tm2 .wav .ogg)
  - at most 8192 entries, 768 MiB per file, 1 GiB in total, counted while inflating

  The zip is staged under `mods/.import`, then the whole mod is checked: `mods::read`, `check`, every glb opened,
  every image decoded, every wav parsed. Only then does it replace `mods/<id>/`. A failed import leaves nothing
  behind, and an installed mod with the same id stays as it was.
- `mods.rs`: `sound()` runs before any glb is used. It catches the input that panicked gltf or gltf-json:
  - POSITION or channel-target indices out of range (gltf-json's validation and `Channel::node()` index without
    checking)
  - accessors that don't fit their view or buffer
  - sparse accessors, wrong attribute types or counts, indices past the vertex count, joints past the skin
  - node loops, nodes deeper than 1024
  - bad inverse bind matrices

  Images and buffers must be embedded. Animation keys: times 0..=3600 s, rotations unit, positions within 1000 m.
- `hst_sim::libm::sinf`/`cosf` return NaN for NaN/±inf instead of asserting, as fdlibm does (`x - x`). A degenerate
  mod arm fed `arm_table` a NaN angle. No finite input's result changes.
- Menu: on wasm, Main → Mods (`play/main_menu/mods_screen.rs`) has Add mod, the installed mods, Reload and Back.
  - Pressing ✕ on a mod arms it; a second ✕ removes it.
  - Taps and clicks choose rows; the right button goes back.
  - The page's file input lies transparent over the Add mod row, placed in window % with `widescreen::share`, so a
    tap opens the picker from a real user gesture (iOS needs that). A key's or pad's ✕ calls `input.click()`.
  - The title tab is a new "Mods" entry in `TITLES`.
- `web/index.html`:
  - At start it walks OPFS `mods/` and drops any folder without its `mod.json` (a save cut short).
  - On a pick it runs `import_mod`, then writes the files to OPFS in parallel, `mod.json` last.
  - If the save fails, it removes the folder and the in-memory copy.

## Proof
- `modfs::tests` (all against MEM, nothing on disk):
  - a plain zip and a zip inside one folder
  - bad archives: traversal, absolute, backslash, device name, symlink, duplicate, bad extension, too many entries,
    an oversized file, a zip bomb
  - `mutated_zips_never_panic`: 20k seeded mutations
  - `real_mods_install`: all 46 mods in `context/mods` install and read back
  - `bad_mods_leave_nothing`: 15 broken copies of Lauryn, each with its own error, nothing left behind and an
    installed mod with the same id untouched
- `mods::tests::mutated_glbs_never_panic`: a seeded mutation fuzz of Lauryn's costume and motions glbs.
  - It changes JSON leaves to edge values, or changes BIN bytes.
  - Each mutant runs through `read`, `check`, `skeleton` and `load`, with the ISO, inside `catch_unwind`.
  - The first runs found the gltf-json panics, then `Channel::node()`, then two `sinf` panics (in `own_motions` keys
    and `arm_table`). All are fixed.
  - The last run, `HST_FUZZ=10000`, had no panics. 4060 mutants passed the checks and 637 loaded fully. The default in
    check.sh is 400.
- `mods_screen::tests`: two-press removal, wrap, Back to Main with Mods selected, scroll, tap hit-test, input rect.
- Headless Chromium (`tools/web_check.mjs`, served copy of `web/` plus a zip of Lauryn; screenshots in scratch):
  - The ISO imports, and Main shows Mods.
  - Keys reach Mods. A zip fed to `#modzip` (change event) shows "Added Lauryn", and Lauryn is listed.
  - At 1280×720 the input sat at 35.9/16.5/28.2/8.5 % (the Add row). On the phone viewport it was 35.6/16.5/28.9/8.5 %.
  - Next visit: "1 custom characters in ./mods", and Lauryn is in OPFS.
  - A random-bytes zip → "Couldn't add bad.zip: not a zip file".
  - ✕✕ on Lauryn → "Removed Lauryn", and OPFS `mods/` is empty.
  - The phone viewport (MOBILE=1, 844×390, touch) lays out fine.
  - A visit cut short mid-save left a folder without mod.json. The next start dropped it.
- Saving is slow in headless SwiftShader, about 0.5 s per OPFS file serially, so the writes now run in parallel.
- `tools/check.sh`: 494 tests passed.

## Not verified / not 1:1
- Real devices: iOS Safari and Android Chrome picking a zip by tap; desktop Chrome importing a mod, reloading,
  selecting it and playing a match. Headless only fed the input's change event; no real picker was opened, and no
  match with an imported mod was played in a browser. Desktop matches with mods go through the same `modfs` reads.
- OPFS `createWritable` on iOS Safari (older versions had only sync access handles in workers). If it throws, the
  status says "Couldn't save … in this browser's storage", and nothing is kept.
- Whether a pad's or key's ✕ opens the picker: `input.click()` without a user gesture may be ignored (iOS
  certainly, Chrome with gamepad input likely). A tap or click on the row always works.
- Memory: every installed mod is held in wasm memory from start (no lazy reads). With many large mods a phone may run
  out of memory.
- The select's roster is built at start, so added or removed mods need Reload (the screen has a Reload row).
- Replacing an installed mod with a new zip of the same id: if the OPFS save then fails, the old copy is gone from
  OPFS too (it was removed first). The in-memory copy is the new one until reload.
- `mods_online.rs` (lobby mod transfer) still unpacks with its own code, not through `modfs::plain`.
- The menu is ours (front-end, not 1:1). The Mods title reuses the Options colour.
