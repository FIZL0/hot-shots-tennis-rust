# B457: web build in a Firefox/Zen private window

## Problem
A Gecko private window refuses OPFS: `navigator.storage.getDirectory()` rejects with "Security error when calling
GetDirectory". The page awaited it at the top level, so every visit stopped on the error screen. Its links (Reload,
`?webgl2`, `?site#settings`, `?forget`) all reloaded into the same failure, which is why they "only reloaded". Copy
the error used `navigator.clipboard.writeText` only, and the private window refuses that.

## Fix
- `web/index.html`: if `getDirectory` throws, `memoryDir()` stands in. It is an in-memory folder with the OPFS calls
  the page and site.js use (get/create file and folder handles, removeEntry, entries/keys/values, getFile,
  createWritable, plus `put(name, blob)`). The site, the game, cards, reports and Settings & files all work on it.
  `writeFile` uses its createWritable. The pack Blob is released once `start` has read it. Saving a mod in memory
  mode says it can't be kept, because a mod needs a reload to play and a reload would lose it.
- `web/worker.js` + `web.rs import_disc_to_memory`: the ISO import without OPFS. The first 1 MB of the pack (its
  header, which gets the hash last) stays in wasm memory. Every later MB goes to JS as soon as it fills, so the worker
  never holds the whole ~171 MB. The page joins the parts into a Blob (`dir.put('disc.pack', …)`).
- `web/site.js`: a note beside the ISO picker and in the storage panel ("pick the ISO again on each visit, settings
  and mods last only until the window closes"). "Keep my data" is hidden in memory mode. `navigator.storage?.` guards
  added. Also fixed: `replaceChildren` showed a literal "null" when the Keep-it row was absent.
- `web.rs read_to_string/write`: when localStorage itself is refused (site data blocked), settings/controls go to an
  in-memory map for the visit instead of failing.
- Copy the error: `copyText` selects a textarea and calls `execCommand('copy')` synchronously inside the click.
  Gecko only allows it there, which an earlier version that awaited the clipboard first got wrong. If that fails it
  tries `navigator.clipboard`. If both fail, the selected textarea stays on screen for a manual copy.
- Error-screen links: with storage no longer throwing, each one does its own thing. Try WebGL2 already passed
  `?webgl2` in the URL and doesn't depend on a saved setting.

## Verified
- Real Zen (`zen-browser --headless`, fresh profile with `browser.privatebrowsing.autostart`), driven over
  WebDriver BiDi (scratch script, not committed). `getDirectory` gives the user's exact error. The page reaches the
  site with the notice, `input.setFiles` with the ISO imports it in memory, the game starts (main menu set up), and
  Copy the error says "Copied". `?forget` and `?webgl2` both reach the site.
- Headless Chromium with `getDirectory` and the clipboard stubbed to reject: same flow. Import, game started (loading
  screen down), Copy → Copied through execCommand.
- `cargo check -p hst --target wasm32-unknown-unknown --release` is clean; `node --check` on the page's scripts.

## Not verified / not 1:1
- Not 1:1: front-end only, no original to match.
- Not checked on a phone's private mode (Safari private has OPFS; only Gecko was reproduced). Memory use there
  (pack Blob + wasm copy) wasn't measured.
- In memory mode, any reload (the game's WebGL2 fallback, GPU lost, update Refresh, Try WebGL2) goes back to the
  site to pick the ISO again. That's inherent to having no storage, and the notice says so.
