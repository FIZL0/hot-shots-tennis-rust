# B470 — web app starts from the cache

Before: `sw.js` was network-first with `cache: 'no-cache'` on every file, so every visit waited on the server (one
round trip per file, and a whole wasm download whenever a new beta was deployed) before the page or the game showed.
Its install list had `pkg/` only, so the WebGPU build (`pkg-webgpu/`) was only cached by the fetch handler.

Now (`web/sw.js`):
- One cache per build, `hst-<version.txt>`, plus `hst-meta` holding which build is `active`, `previous`, `ready`.
- Every same-origin GET is served from the active build's cache without touching the network; a miss is fetched and
  kept. Requests with `cache: 'no-cache'` (site.js's `stats.json`) stay network-first; `no-store` / query requests to
  the server aren't cached.
- `version.txt` is answered by the SW with the active build; index.html pins `pkg*/hst.js` and `hst_bg.wasm` to it
  with `?v=<build>`, served from that build's cache (a miss is only stored if the server is still on that build), so
  hst.js and its wasm can't mix builds. Whichever renderer's pkg the page loads gets cached, WebGL2 or WebGPU.
- Each navigation and each `version.txt?next` poll (index.html: load, every 10 min, tab visible) starts a background
  download of the deployed build: the shell plus every URL the active build has cached (so the renderer pkg in use,
  pictures). It counts as ready only when all of it is in and `version.txt` didn't change meanwhile; `?next` then
  returns it, index.html sets `hstLatest`, the menu's New version card (B317) appears, and its Refresh reload
  (any navigation to the page) promotes ready → active. The previous build stays cached for tabs still loading it;
  older ones and the old single `hst` cache are deleted.

Check: a headless-Chromium harness (scratchpad, a Python server faking builds A/B and a driver page stepping through
reloads) passed: controlled after register, A served from cache after B is deployed, `?next` reports B once
downloaded while the open page keeps getting A's wasm, reload opens B (page, hst.js, wasm), server down → B still
loads from cache.

## Not verified / not 1:1
- Not run on iOS Safari / WebKit (no device or WebKit driver here); only headless Chromium with fake files.
- Not run against a real `tools/web.sh` build of the game (the harness used stand-in hst.js/wasm).
- First visit: the page isn't controlled by the SW yet, so that visit's pkg is cached on the second visit (as before
  for pkg-webgpu); offline play needs two visits.
- Menu/front-end behaviour, so no original to compare with.
