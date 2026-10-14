# 1 — Desync reports piling up (FINAL)

## What the error is
"Caught rendering error: Out of Memory" is bevy_render's uncaptured wgpu error handler (`error_handler.rs`): an
`OutOfMemory` from wgpu; the default handler quits the app, and `web/index.html` shows "The game stopped". So some
allocation failed — wasm heap or GPU — not necessarily anything the wait pop-up itself does.

## Measurements (runs in `context/B276/`)
`HST_FPS=1` now also logs, every 5 s, asset counts (images + CPU kB, meshes, standard/fx materials), entity count
and the wgpu allocator totals (`main.rs log_assets`).

- Native bot match 100–240 s (run1–run4): assets, entities, wgpu allocated (~260–270 MB) / reserved (590 MB)
  all flat.
- Native online match, host + guest (`hst net host/join --relay off --play`, `HST_AUTOPLAY`), guest frozen with
  SIGSTOP 5× for 12 s (the wait pop-up: "went silent" / "answering again") (net1, net2): flat, also the wgpu-hal
  object counters (buffers ~4000, bind groups 1000–1800, textures, views) checked with a temporary build.
  The wait pop-up (`main_menu/pause.rs` notice) draws from a fixed pool, no per-frame assets.
- Browser (headless Chromium, SwiftShader WebGPU, `tools/web.sh` build), bot match 240 s (web1): GPU process
  RSS 290–319 MB sawtooth, flat.
- Browser guest joining a native host by short code (wnet2): the test's host drove the guest's seat as a bot too,
  so the match went out of sync every 30 ticks, the guest took the host's state (B249) and played on — and each
  cycle **saved a new desync report**: 40 `desync-*.json` in ~400 s, each the whole pad log so far (54 KB →
  243 KB, quadratic over a match), each logged whole to the console by `web.rs save_file`, written to OPFS
  `reports/` and offered as a download (Blob URL). That is the growth: wasm heap, console and blob/OPFS memory
  climb with every resync until something fails to allocate — in the user's case mid-wait, when the renderer next
  asked for memory.

## Cause
`netplay.rs desync` meant to save "once" but only skipped while the 6 s note from the last one was up; after it,
every new out-of-sync hash saved the whole input log again.

## Fix
`Netplay.report` keeps where the match's one report went; `save_desync` returns it instead of saving again, so
later desyncs are still logged and noted ("Inputs saved to <same path>") but write nothing.
Test: `a_desync_is_saved_and_play_goes_on` now forces a second desync after the note is down and checks the note
names the first report and no file is written.

## Not verified / not 1:1
- The user's actual OOM wasn't reproduced: only headless Chromium with SwiftShader here, no real browser/phone GPU,
  and `context/B276/oom.png` didn't exist. That the user's match was resyncing repeatedly is inferred (it's the
  only growth found), not shown.
- Browser memory after the fix wasn't re-measured over a full match with waits (the fix removes the per-resync
  save outright; the native runs with waits were already flat).
- Going match → lobby added ~260 MB wgpu reservation natively (menu assets loading); not investigated, it isn't
  per-match growth.
- Web-only presentation, nothing to match against the original.
