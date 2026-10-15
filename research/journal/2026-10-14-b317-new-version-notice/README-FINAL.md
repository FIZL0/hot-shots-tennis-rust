# B317 — new version notice (web Refresh, desktop Download)

## What changed
- `crates/hst/src/update.rs` (new): `latest()` = the newer build's commit, if one is out; `act()` = Refresh
  (`web::reload`, which waits for OPFS mod saves) on the web, open the `alpha` release page on desktop.
  - Web: `web/index.html` reads `version.txt` (written by `tools/web.sh`: `git rev-parse --short=7 HEAD`) before the
    game loads, then every 10 min and when the tab comes back; a different value sets `window.hstLatest`.
  - Desktop: only builds made by GitHub Actions (`GITHUB_SHA` at compile time) ask
    `api.github.com/repos/FIZL0/hot-shots-tennis-rust/releases/tags/alpha` for `target_commitish` at start and every
    30 min; local/dev builds never ask (their commit is never the alpha's, they'd nag forever).
  - `HST_LATEST=<commit>` fakes one natively (screenshots); `HST_MENU=main-update` highlights the card.
- Main menu (`home.rs`): `Row::Update`, a gold "NEW VERSION OUT / Refresh|Download" card under the rows, reachable by
  pad/keys and pointer; its preview shows NOW → NEW commits, words and two facts. `main_menu.rs` dispatches it.
- `web/sw.js`: comment only. Nothing in the page or the service worker reloads the page when a build is pushed;
  the SW is network-first, so Refresh (a plain reload) fetches the new build. `/web/version.txt` git-ignored.

## Checked
- Native shots at 16:9 and 4:3 (`context/b317/main.png`, `main-update.png`, `update-43b.png`): first 4:3 shot
  overflowed the version tiles and cut the words; tiles now scale to the panel, words shortened.
- `cargo check --target wasm32-unknown-unknown` builds.

## Not verified / not 1:1
- The page polling in a real browser (no wasm build/deploy in this run): the JS is small (fetch `version.txt` with
  `no-store`, compare, set `hstLatest`) but wasn't run headless.
- The desktop GitHub check against the real API: only Actions builds query it, none was made here.
- Where the "page reloads itself on a new build" the task mentions came from: nothing in index.html/sw.js reloads on
  a deploy (only the GPU-device-lost, renderer-change and WebGL2-fallback reloads, which stay). Not reproduced.
- Menus are ours (not 1:1 by rule).
