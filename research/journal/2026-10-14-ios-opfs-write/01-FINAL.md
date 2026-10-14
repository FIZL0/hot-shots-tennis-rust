# B358 — mod saves on iOS Safari (no createWritable)

**Problem.** iOS 18.6 Safari (home-screen webapp) failed every mod save with
`createWritable is not a function`. Safari before 26 has no `FileSystemFileHandle.createWritable` on the
page. The only way it can write an OPFS file is a dedicated worker's `createSyncAccessHandle`. The disc import
already ran in a worker (`web/worker.js`), so it was never affected. The mod save (`hstModSave`), the select cards
(`hstCard`) and the desync reports (`hstSaveFile`) all called `createWritable` on the page.

**Fix.** `web/index.html` has one `writeFile(path, bytes|text)`:
- If `createWritable` exists, it writes on the page as before.
- If not, it posts the path and a copy of the bytes (transferred) to `web/opfs.js`. That small worker makes the
  folders, then truncates, writes, flushes and closes a sync access handle, and answers with the id or an error.
  It awaits `truncate`/`write`/`flush`/`close`, because early WebKit versions returned promises from them.

All three writers use it. The mod save keeps its order: up to eight files at a time, then `mod.json` last.
On an error it still cleans up as before. `opfs.js` is in the service worker's offline shell. modfs.rs,
modstore.rs and mods.rs needed no change: the Rust side only hands bytes to the page.

**Check.** `node tools/web_opfs_write.mjs` runs headless Chromium twice: once as is, and once with
`createWritable` deleted from the prototype (the Safari path). Each run writes a 3 MB nested file, a text file,
a card and a shorter rewrite of the text file, all at the same time, then reads them back. Both runs: PASS.

## Not verified / not 1:1
- Not run on real iOS Safari or WebKit. No iPhone is reachable, and Playwright's cached WebKit (2083) won't
  launch with the installed playwright-core (it wants 2359). The Safari path was checked in Chromium with
  `createWritable` removed. That uses the same worker API (`createSyncAccessHandle`) that WebKit supports since
  15.2, but WebKit's own behaviour is not proven. The user should try a store download on the iPhone.
- 1:1 doesn't apply: this is front-end/storage, not gameplay.
