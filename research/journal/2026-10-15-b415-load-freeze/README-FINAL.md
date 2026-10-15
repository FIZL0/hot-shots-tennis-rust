# B415 — loading freezes the app ("Not responding")

## Cause
Not the disc import: on desktop it runs before the window exists (`disc.rs`), on the web in a worker.
The freeze is `mode.rs::switch`: it ran the new mode's PreStartup/Startup/PostStartup in **one frame**.
Chrome trace (`context/b415/run1`, RX 9070 XT, `--play`): the match build was a single 1.18 s frame
(play::setup 313 ms, pause::setup 264, menu::setup 169, match_stats::setup 168, load 153); menu boot
main_menu::setup 860–1244 ms, ~772 ms of it `menu_art::image` rebuilding ~60 sheets one after another.
In the throttled browser (THROTTLE=4) the same build is ~10.6 s of work — one frame, so the page hangs.

## Fix
- `mode.rs`: the build is paced. Each startup schedule runs single-threaded with a run condition on every
  system; per frame the first system always runs, then more until a 25 ms budget is used, the rest wait for the
  next frame (`Building` resource, `step`). Stages stay in order; the mode's Update only starts when all three
  have finished (`Running`), and `Loading.built` is set then, so the loading screen stays up and animates
  between steps. Logs `mode built over N frames in X s (longest Y ms)`.
- Meshes created empty during the build (effects/rain/sprite meshes the mode's first Update fills) are kept out
  of the render world until the build ends (`hold_empty`): extracting them between paced steps gave Bevy
  "copy element data for an unallocated key" errors and gs.wgsl "invalid field accessor position".
- `menu_art::images` builds a batch of menu sheets on all cores (`textures::parallel`, serial on wasm); used by
  main_menu, pause, menu and match_stats setup.
- Test `mode::a_slow_build_spreads_over_frames`.

## Numbers
- Desktop match: 1373 ms slowest frame → built over 7 frames in 1.23 s, longest ~290–360 ms (play::setup alone).
- Desktop menu boot: main_menu::setup 860 → 421 ms.
- Menu → match through the real menu (HST_KEYS): built over 7 frames, no WARN/ERROR.
- Web, THROTTLE=4 (`tools/web_check.mjs … play 60`): built over 10 frames in 10.61 s, longest 1974 ms
  (before: one ~10.6 s frame). Visit 1: 6 frames in 4.17 s, longest 524 ms. The 3.3 s frame at ~5 s is
  headless SwiftShader losing WebGPU and reloading with WebGL2 (known, see 2026-10-12-m3-browser), not a load.

## Not verified / not 1:1
- Phone: no device/emulator log in this run; the throttled browser stood in for a slow device.
- Windows "Not responding" dialog not reproduced directly; frame lengths measured instead (well under the ~5 s
  Windows hang threshold now on desktop).
- Single systems are still long: play::setup ~300 ms desktop / ~2 s throttled web. Splitting it means editing
  play.rs, which other agents edit concurrently; left for P50/P47/B409.
- Loading/menus are ours to design: no comparison with the original needed.

## Merge note (master, 2026-10-15)
P50 landed the same pacing on main first (`Starting`/`Sliced` executor, 20 ms slices, menu sheets drawn on the async
pool). The merge kept P50's executor and menu_art, dropped B415's `Building`/`pace`/`step`, `menu_art::images` and its
callers, and its test (P50's `a_long_start_is_spread_over_frames` covers it). B415's `hold_empty` (empty meshes kept out
of the render world until the start ends) was ported onto `Starting` in `mode.rs`.
