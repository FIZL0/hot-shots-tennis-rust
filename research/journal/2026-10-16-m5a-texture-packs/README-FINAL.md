# M5a — texture replacement packs as a mod kind (FINAL)

## What landed

- `hst-base/src/mods_textures.rs`: a pack is `mods/<id>/mod.json` (`"standard": 1, "kind": "textures"`, id/name
  1–64 chars, optional author) + `textures/**/<anything>-<key>.png`, the `--dump-textures` names (key = the 16 hex
  digits after the last `-`; the first file of a key in path order wins). A pack with no textures or a bad manifest is
  refused and logged. The order is `Order { order, off }`, kept in settings.txt as `texture_packs = a, b` (top first) and
  `texture_packs_off = c` (folder names under `mods/`, `..`/absolute names dropped). An unlisted pack is on, above the
  listed ones, so a new install lands on top.
- `mods.rs` `Kind::Textures` (`read_any` reads it, `read` refuses it as a character, off the roster); `modstore.rs` maps it.
- `textures.rs`: lookup is packs (top first) → `mods/texture-replacements/` (key-named → PCSX2 pack; desktop only) →
  disc. Pack files are read through `modfs`, so the browser's in-memory OPFS copies work. This also takes away B227's
  "no texture replacements on web" lock: the browser now reads packs, but still never `texture-replacements/` or mods'
  `textures/upscaled/`. Switching the setting back on re-reads only the packs. Reload skips keys a pack covers.
- `modfs.rs`: `unload()` keeps a texture pack's files (the menu's textures are replaced at the start, outside any
  mod load). `web/index.html` reads a `kind: "textures"` mod in whole at start, not lazily.
- Settings: "Upscaled Textures" → "Texture Replacements" (label, descriptions, home fact, pause doc, README,
  DEVELOPING.md, the web page's blurb). Mods' `textures/upscaled` and B227's Upscaled Downloads keep their names.
  `Settings::parse`/`text` and `main.rs start()` carry the order. The Mods screen's Texture packs empty state says
  how to install one.
- `--dump-textures` message and main.rs module doc name packs.
- ../opentennis-mods `standard/HST-TEXTURE-PACK-STANDARD.md` (new) + the court/umpire §1 kind table + README
  (commit 12b80c7 there, **not pushed**: the run says don't push, and that branch also holds another agent's unpushed
  B488 commit).
- `tools/web_zen.mjs`: `WEB_PRELOAD=js` runs before the page's scripts (used to put the packs into OPFS).

## Verification

Two packs made from `--dump-textures` output (`context/m5a/run1/`, a 2× resize + tint of each source PNG):
`pack_hi` = court 1 ground (GRD01) in red; `pack_lo` = the same files in blue + court 1 trees/net (HOL01) in magenta
(+ the CMN backdrop in green on desktop).

- Desktop (`tools/shot.sh`, `--play --stage 1 --court 0 --shot-at 2`, own settings.txt; `context/m5a/run1/grid.png`):
  default → red ground + magenta trees (hi wins, lo fills the rest); `texture_packs = pack_lo, pack_hi` → blue;
  `texture_packs_off = pack_hi` → blue; `--no-upscale` → disc look; `--original` → disc look, 4:3.
- Browser (Zen, WebGL2, `tools/web_zen.mjs`, packs written into OPFS `mods/` by a `WEB_PRELOAD` script;
  `context/m5a/web-grid.png`): default → red ground + magenta net/trees; settings.txt (localStorage)
  `texture_packs = pack_lo, pack_hi` → blue; `texture_packs_off = pack_hi` → blue; `?no-upscale` → disc;
  `?original` → disc, 4:3. No "game stopped" on any of those runs.
- Tests: `mods_textures::tests` (keys from dump names, reading/refusing a pack, order + settings round trip),
  `textures::tests::packs_over_replacements_over_disc` (layers, order, off, an in-memory browser pack),
  `modfs::tests::texture_pack_installs_and_stays_in` (zip install, kept through `unload`), mods kind tests,
  main_menu options/mods_kind/mods_screen.

Harness pitfalls, for the next web check: a preload that writes OPFS while the page starts races its mod scan. The
page drops a mod folder that has no `mod.json` yet (a save cut short), and Firefox threw `UnknownError` on one
`createWritable`. The first preload let that rejection go unhandled, so the page's `unhandledrejection` handler stopped
the game. Write the files in a run of their own (catch everything, skip files already there), then check in a visit
without the preload. settings.txt on web is localStorage `./settings.txt`.

## Not verified / not 1:1

- Not against the original: the original has no texture packs. This is an addition. With no packs installed, or the
  setting off, nothing changes (lookup falls through to the old layers), and packs never touch the sim.
- Chromium (`tools/web_check.mjs`) and the WebGPU build: not checked. `web/pkg-webgpu` was not rebuilt. One early Chromium run
  showed a black page; I didn't chase it (probably the same preload race as below) and switched to Zen. Zen/WebGL2 only.
- Installing a pack through the browser's Add mod *button* was not driven. The zip path is covered by
  `texture_pack_installs_and_stays_in`, and the packs went into OPFS directly.
- The browser keeps every pack's files in wasm memory for the session, PNG bytes, compressed (B227's memory concern
  in miniature). Big 4× packs will cost that much RAM in a tab. Not measured with a big pack.
- No UI for order/on-off yet (M5b): settings.txt by hand. A change to the order applies from the next start (textures
  already in use aren't swapped mid-session).
- The Mods screen's Texture packs empty-state text is only reachable when that tab is chosen with nothing in it. Its
  layout was not shot.
- Menus are ours (PLAN): the setting's name and descriptions have no original to compare against.
