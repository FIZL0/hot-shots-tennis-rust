# B120 — mobile select: T-pose and freeze on hover before the cached card

## Cause
- The browser had **no card cache at all**: `bake` returned at once on wasm (B102's `ponytail:`), so every hover on the
  web loaded and spawned the live model (a load inside a frame: ~10 ms desktop, ~30 ms+ on a phone) and the card only
  ever showed the live rig.
- A live rig's joints spawn at rest (T-pose). `character::animate` is unordered with `sync`, so when the spawn's
  commands land after that frame's `animate`, the first frame draws the T-pose. Natively a PNG covered it (COVER
  frames); with no PNG (the browser, or natively before the bake) it showed.

## Fix
- `inspect.rs`: a preview's game space spawns `Visibility::Hidden` with `Unposed`; `reveal` (after `centre`, so after
  `animate`) shows it the second time it sees it, i.e. after an `animate` that surely ran on it. No T-pose frame.
- `previews.rs`: the baker runs in the browser too; cache access goes through `cached`/`read`/`write` (files natively,
  OPFS via `web.rs` on wasm, names `<SCUS_976.10-r3>~d00-0.png`, older drawings' dropped). PNGs encode with fast
  deflate (the encode runs inside a frame in the browser; 141 cards = 11 MB). In the browser a held hover no longer
  loads the live model (the card is the same held pose); a press (R2/L2/△) still does.
- `web.rs` + `index.html`: the page reads OPFS `previews/*` into the game (`add_card`) before `run`; new cards are
  written back by `hstCard` (`createWritable`, committed on close), stale ones removed by `hstCardDrop`.
- `tools/web_check.mjs`: `MOBILE=1` = phone-sized landscape touch viewport (as web_touch.mjs).

## Verified
Headless Chromium on the real GPU (`CHROME_FLAGS="--enable-features=Vulkan --use-angle=vulkan"`, `MOBILE=1`), the same
seeded walk as B102 (3× Enter into the select, then 4 random arrows / `e` a second for ~40 s; `context/b120/walk.txt`),
frames after second 16. Runs in `context/b120/`.

| run | cards baked | frames | > 33.4 ms | worst |
|---|---|---|---|---|
| WebGL2, first visit (empty cache) | 140 | 4603 | 4 | 99.9 ms |
| WebGL2, second visit | 0 (all read from OPFS) | 4602 | 0 | 16.8 ms |
| WebGPU, first visit (empty cache) | 140 | 4614 | 6 | 283 ms |
| WebGPU, second visit, no screenshots | 0 | 4455 | 0 | — |

The second visits bake nothing: the OPFS cache is hit. (A WebGPU second visit *with* screenshots every 3 s had 30–78
long frames, also before the select was reached: `Page.captureScreenshot` stalls and a load average of ~46 from the
other agents; without screenshots, 0.) Native: `HST_BAKE=all` from an empty cache bakes all 141 cards, and the
`HST_PREVIEW=live` and `=png` shots of four cards match; `tools/select_stress.sh 20`: no panic, 0 card-frames showing
a wrong key; `tools/check.sh -p hst` passes.

## Not verified / not 1:1
- No real phone: a desktop GPU with phone emulation only; the first visit (cards baking in the background) still has a
  few long frames (the first bakes' pipelines, a load + bake per card), on a phone likely more. Later visits don't bake.
- The "no T-pose frame" is from the ordering argument and the code path, not caught on a recorded frame: headless
  screenshots every few seconds can't see a single frame.
- New native/web bakes show a neutral face where the main checkout's older cache (baked 12:08, before B102) has a
  grin; the live model with this code matches the new cards, so it predates this task (B102's motion subset or
  earlier) — not checked against the original's inspect screen here.
