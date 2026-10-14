# B338 — Reload "did nothing" on mobile after store installs

## Cause (from the code)
- A store install is in memory when `Event::Installed(Ok)` arrives; `mods_screen.rs` marks mods changed right then
  (`View::changed`), so leaving Mods puts up B205's Reload / Later popup at once.
- The OPFS write is the page's `hstModSave` (`modstore.rs keep` → `web::mod_save`), not awaited by the game: on a
  phone a mod's many files take seconds, and `mod.json` goes last.
- Reload = `location.reload()` straight away: the save is cut short, the next start finds `mods/<id>` without its
  `mod.json` and drops it (index.html's startup walk). The page did reload, but the mod wasn't in: "did nothing".
  Once the save had finished (the next try), Reload worked — matching the user's "intermittent, right after an
  install". The same race applies to a Remove (`hstModDrop`'s recursive removeEntry) cut short.
- `location.reload()` itself works on iOS Safari / Android Chrome (it's what the boot screen's own reload uses); no
  sign it was refused.

## Fix
- `web/index.html`: `hstModSave` and `hstModDrop` are tracked in a `pending` set; `hstReload` waits until it's empty
  (re-checking for saves started meanwhile, at most 2 min so a hung write can't swallow Reload), showing "Saving mods
  before reloading…" on the boot overlay while it waits, then `location.reload()`.
- `web.rs reload`: calls the page's `hstReload` (falls back to `location.reload()` if the page lacks it).

## Checks
- `cargo check -p hst --target wasm32-unknown-unknown`, `node --check` on the page module.
- The wait loop run in Node with fake saves: waits for a save started during the wait (400 ms), gives up at the cap
  with a hung one.

## Not verified / not 1:1
- Not run on an iPhone / Android phone or in a browser (no device here; needs the ISO in OPFS and a store install):
  the cause is read from the code, not reproduced. If Reload still does nothing with no save under way, the next
  suspect is the page itself (e.g. Safari ignoring the reload), to look at on a device with the console.
- Front-end, ours by design: no original counterpart.
