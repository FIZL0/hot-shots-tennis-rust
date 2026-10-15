# P50 — iOS load hitches: start spread over frames behind the loading screen

## What the build 2316 log says
(`context/p49/hst-log-build2316.txt` in the main checkout)

- Menu boot: `game stall: update 685.7 ms` at 0.81 s; the cover came down 12 frames later at 1.06 s.
- Match start: one `update 775.2 ms` frame (disc +78.3 MB, +2263 entities) at 45.82 s, then `render 172.1 ms`
  (images +326, meshes +266, ready +47, compiling 18), then a 159 ms gap. Pipelines kept landing behind the cover
  until 47.68 s, and the cover came down after 100 frames at 47.72 s.
- **Every frame after the cover came down took 14–17 ms, and nothing was compiling.** The existing warm-up in
  `mode.rs` already pre-warms the match's pipelines behind the cover, so P50 adds no extra pipeline warm (P47
  covers the rest of that family). The hitch the user sees is the loading screen itself freezing: `switch` built
  the mode and ran all of PreStartup/Startup/PostStartup in one frame, so the still loading screen froze for
  0.7–1 s ("Not responding", B415).

## What changed
- **Sliced start** (`mode.rs`): `switch` now only builds the mode. Its start schedules each get a `Sliced` executor
  that runs one system per pass (the others are skipped through the executor's skip list, which needs bevy's
  `bevy_debug_stepping` feature, now on in `crates/hst/Cargo.toml`). Systems keep running until the frame's 20 ms
  slice is used up, then the start continues next frame. Commands are applied after each system, as before.
  The mode's Update waits (`Running` is only inserted, and `Loading.built` only set, once the start is through),
  so the loading screen stays up and the frame loop keeps going. A start system over 50 ms logs
  `start: <system> X ms`; the end logs `started in N frames, X ms at T s`.
- **Deferred menu art** (`play/menu_art.rs`): during a start, the sheets we rebuild (word, option_00,
  result_*, …, ~510 ms of the 1170 ms match start) are drawn on the async pool under a reserved handle and land in
  `First` (`menu_art::land`). The start isn't through until every sheet has landed. Same pixels: the pure part is
  `rebuilt()`, and the test `a_deferred_sheet_lands_the_same` compares the two paths.
- **Early character load** (`preload.rs`, `Mode::early`): when the menu asks for the match, the match's disc
  characters start loading on the async pool into `Staged` stores, while the menu fades and the loading screen goes
  up. `play::setup` takes them through `preload::load_disc` (one-line change in play.rs), which uses the same loader
  and data as before. The sliced executor waits (`preload::busy`) instead of running the next system while they are
  still loading. Modded slots aren't preloaded and load as before.

## Measurements (desktop, release, `--play --stage 1`, HST_FPS=1)
| | frames | total | longest frame |
|---|---|---|---|
| match start before | 1 | 1.2–1.8 s | the whole start |
| sliced only | 7 | 1223 ms | play::setup 349 ms |
| + art deferred | 37 | 1020 ms | ~170 ms |
| + characters preloaded | 32 | 709 ms | `hst::load` (court) 149 ms |
| menu boot | 3 | 494 ms | main_menu::setup 189–298 ms |

Screenshots after the change (`context/p50/run4.png`, `menu2.png`) show the full match (four characters, HUD
art) and the main menu, drawn as before. Tests: 6 mode tests (new `a_long_start_is_spread_over_frames`) and
5 menu_art tests pass. wasm32 `cargo check` passes.

## Not verified / not 1:1
- **No device log yet.** The fix is measured on desktop only. The next iOS log should show `start:` /
  `started in` lines and no 700+ ms update stall. The task asks for this check with the next device log, which needs
  the user's device.
- **Still single frames:** `hst::load` (court models, ~100–150 ms, a 335-line function in main.rs that other
  agents edit, not split), `main_menu::setup` (~190–300 ms: reads, `textures::prefetch_tim2`, the sheets that
  aren't rebuilt, mods) and `previews::start_baker` (105–137 ms). On a phone these may still be 0.3–0.6 s frames.
  A system is never cut in the middle, only spaced out.
- The menu → match early preload was exercised through `--play` (the same `Mode::early` path), not by driving the
  menu.
- Web: the art and character tasks run on the main thread between frames there, so the time moves to the frames
  around the fade instead of disappearing. Not checked in a browser. An unfinished preload falls back to the
  synchronous load, so it can't deadlock.
- iOS build not compiled here (same native code path as desktop aarch64; wasm32 check passed).
- Loading screen / menus don't have to match the original (PLAN.md); no original comparison needed.
