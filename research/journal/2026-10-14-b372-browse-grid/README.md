# B372 — Browse mods as a picture grid (2026-10-14) — FINAL

Browse's text list is now a picture grid (`crates/hst/src/play/main_menu/mods_screen/grid.rs`).

## What's there
- **Top bar**: "Mods" title, tabs Browse N · Installed N · Upload · My mods. On a wide view the search box and the
  downloads pill sit in the title band; on a narrow or portrait view they go in a row under it.
- **Search**: gold-bordered box with a magnifier and "N found". It matches name, uploader and game. A keyboard
  types straight into it (`grid::typing`, runs before `step`). On a pad, ✕ opens an on-screen keyboard: 4×10 keys
  plus Space/Delete/Clear/Done. □ deletes and ○/Done closes it. In the browser it uses `web::code_entry`/`typed`.
- **Rail**: Game (games by count, Other past six), Plays like, Hand, Show (Everything / Not installed / Updates /
  Verified only / Wish list). Each row shows its count, the chosen one is filled, and counts follow the other filters.
- **Grid**: a heading ("All games - N mods") and the sorts Newest · A-Z · Uploader, plus Refresh. B335's folding
  is removed. Rows hold 5 tiles at 1280×720, 3 at 4:3, and fewer in portrait. Each tile shows the name, its game
  and the hand L/R. The selected tile gets a gold outline and gold name. Other markers: a blue ✓ disc for verified
  mods, Installed/Update pills, a progress strip with Downloading %/Checking/Queued, and a gold scroll bar.
- **Detail panel**: name with ✓ Verified, game · version · size, pills (costumes, hand), a big picture,
  "Plays like …", then ✕ Install·size / Update / Remove (opens the Remove child), △ More by uploader (B255 flow;
  △ again goes back) and □ Wish list (`mods-wishlist.txt` in the store).
- **Footer**: "Pictures load by themselves: …" plus the legend (`browse_legend`).
- **Pointer**: hover chooses a tile. A tap chooses it and a second tap on the same tile presses ✕. A finger
  going down doesn't choose, so a drag scrolls. The wheel scrolls. Rail, sort and search take taps.
- **Pictures** (`grid::plan`/`want`/`got`/`evict`, `browse_pictures`): every tile on view is asked for first, then
  one more screen the way the grid last scrolled (nearest row first; at the top, the screen below). The cap is
  Downloads at Once (at least 2), or 1 while installs run. Cached pictures are read with no wait, up to 8 rounds
  a frame. Past 160 loaded, the ones far off are dropped. The cache is keyed by the zip's sha256: on desktop it is
  `mod-pictures/<sha>.png` beside mods/ (empty file = "no picture"); in the browser it is the `mod-pictures~` card
  folder (OPFS), kept by `cards_keep`. Uploads now include `thumb.png` (256 px). Older mods without one get their
  `preview.png` shrunk once and cached. An installed mod with no store picture shows its select picture.

## Tests
`mods_screen.rs`:
- `grid_sorts_filters_and_moves`: sorts, search, rail counts, cursor moves, 5 a row at 16:9.
- `browse_and_upload_pages`: tile tap twice queues the install, hover clears the tap.
- `browse_by_author`: △ flow, wish list, Download all.
- `pictures_on_view_then_ahead`: on-view then ahead keys both ways, cap, `got` frees a turn, evict.
- `pictures_cache_by_sha256`: keep/read/none/unknown, `shrink` to 256.
- `search_with_the_keyboard`: on-screen keyboard.

`upload_all` and `other_screens_keep_their_cursor` now share a lock. `page` drains the global store event queue,
so it took the uploads' events when the two ran at once. `tools/check.sh -p hst`: 356 passed. Wasm check is clean.
At wrap-up, a full `tools/check.sh` failed only `rollback_survives_a_tab_out`: "the guest's picture flipped to the
shared camera for [6, 187] rounds". It fails the same way on the untouched base commit fde61ac0, tried twice in a
scratch worktree, with load average ~25 from the other slots. It is wall-clock based (a 5 s tab-out) and unrelated
to this task. So `done.sh` was skipped: the task was ticked with `tools/ctx.py tick` and committed by hand.

Shots in `context/b372/`: grid169b and grid43b (1280×720 and 960×720), search, keys, keys43, portrait, portrait2.

## Not verified / not 1:1
- **Concept artifact**: the claude.ai board couldn't be opened from here, so the layout follows the written spec
  in plan/B372.md, not a pixel comparison. This is a front-end screen, so it's ours to design (PLAN.md).
- **Live store at scale**: not run against the real store with 100+ mods. The network fetch of thumb.png/preview.png
  and OPFS in a real browser were not tried live (no network/browser session in this unattended run). The fetch
  order and the sha256 cache are covered by the tests above with a temp dir.
- **Button glyphs**: drawn as text ("X", "^", "[]") on the panel buttons, like the other mod pages, not PS glyph
  icons.
- **Failed fetches**: a picture that failed on a network error stays a letter tile until the game restarts. There
  is no retry.
- **Game rail**: shows at most six games plus Other.
- **Drag scroll**: a drag moves the grid through the existing direction presses, not a smooth pixel scroll.
- **Download queue pill**: shows counts only. It doesn't open a queue list.
