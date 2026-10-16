# P60 — split the hst crate (FINAL)

Goal: an edit rebuilds and relinks less. Pure move, no behaviour change.

## What moved

New workspace crate `crates/hst-base` (~22k lines), the layer under the match and menus: audio, character, graphics,
gs/shade/shadow/textures/lod/cull, mods (+ modfs, modstore, mods_online, mods_umpire, mods_court, court_mod), net
(+ net/), chat, handshake, inspect, jobs, stall, effects, weather, noise, spring, ui_pool, ui_census, scene_census,
vertex_patch, hud_gamma, the platform glue (web, ios, android, android_pad, disc) and `widescreen` (was
`play/widescreen.rs`). From `main.rs`: `Args`, `Orbit`, `GameSpace`, `Backdrop`/`BACKDROP`, `store`, `fps_log`.

`hst` keeps the top: `main.rs`, `play.rs` + `play/` (match, netplay/rollback/snapshot, `main_menu/`), `mode.rs`,
`loading.rs`, `preload.rs`, `mash.rs`, `sandbox.rs`, `update.rs` (needs `HST_COMMIT` from hst's build.rs).
`main.rs` does `pub use hst_base::*`, so every `crate::audio`, `crate::net`, … path in hst still resolves.

How the 21 base→top references were cut (found with a file-level dependency graph, none left):

- `Seat`/`readable`/`ignore_list` → `hst_base::seats` (controls.rs re-exports them); `Wire` → lib.rs
  (snapshot.rs re-exports it); the mods folder root → `modfs::{root, set_root}` (mods_screen re-exports them);
  the settings file path → `store::settings`.
- `graphics::plugin::<Game>`: the auto-tune runs while the match resource exists (was a `match_on()` call into play);
  saving its pick goes through `graphics::SAVE` (a `OnceLock<fn>` main.rs fills with main_menu's `save_graphics`).
- Android's background/pause system moved from `android.rs` into `play/background.rs` (it touches the match);
  the wasm entry `run()` moved from `web.rs` into `main.rs`.
- `stall::diag!` is `#[macro_export]`; embedded shaders are now `embedded://hst_base/…`; `mode.rs`' teardown
  recognises both crates' type names (`OURS`).
- Test helpers hst's tests call (audio `bare`/`test`/`endless`/`plays`/…, net `wired`/`enter_room`, modfs
  `unpack`/`install`/`Files`) are `#[cfg(any(test, feature = "testing"))]`; hst's dev-dependency on hst-base
  turns `testing` on, the game build doesn't.
- The version lives once in the root `Cargo.toml` (`[workspace.package]`); `tools/release.sh` and
  `tools/ios-ipa.sh` bump/read it there.

## Measurements

Edit-one-file cycle: change one string literal, `cargo test -p hst --no-run -j 5` (check.sh's job count on this
16-thread box), three runs each, before (main 97f9a19d) and after interleaved, load ~3:

| edited file | before (s) | after (s) |
|---|---|---|
| `play.rs` | 20.7 20.9 20.9 | 11.6 15.5 15.5 |
| `play/main_menu.rs` | 26.3 26.0 26.1 | 14.3 14.3 14.1 |
| `net.rs` (now in hst-base) | 25.7 25.7 25.9 | 17.9 17.8 18.3 |
| `net.rs`, `cargo test -p hst-base --no-run` (new) | — | 6.0 5.7 5.6 5.6 |

So ~21 → 15 s for the match, ~26 → 14 s for the menus, ~26 → 18 s for a base file, and ~6 s when a base-only change
is checked with hst-base's own tests (143 of the 490 tests live there now).

Same 490 tests before and after (`--list` of hst before = hst + hst-base after). Passed: `check.sh -p hst-base`
(143), `-p hst` (313), `HST_NETPLAY=1 -p hst netplay` (26), `-p hst-sim replay`, `-p hst-sim --test platforms`.
wasm32 release build: all 17 `#[wasm_bindgen]` exports (16 in hst-base's web.rs + `run`) are in the generated JS.
Android: `cargo check --example hst --features android --target aarch64-linux-android` clean.

## Not split further

The menus (`play/main_menu/`) aren't self-contained: the in-match pause menu embeds the options pages, and the
match core reaches menu types (netplay/background → menu, snapshot → match_stats/panel, intro → pause labels,
recover/netplay → pause notices, menu_art → main_menu's font). A `hst-play` crate under the menus needs those
untangled first: P61.

## Not verified / not 1:1

- iOS not compiled: `cargo check --target aarch64-apple-ios` stops in blake3's C build script (needs Xcode's
  `xcrun`). The iOS-only code (`ios.rs`, its callers in main.rs/graphics.rs/widescreen.rs/touch_pad.rs/disc.rs/
  modstore.rs) was read through: everything it names is `pub` and reaches hst through the re-export.
- Android checked, not built into an APK or run on a device.
- The wasm build wasn't run in a browser, only checked for its exports.
- `rollback_desync_is_saved_and_play_goes_on` failed twice (2 resyncs, 1 expected) in `HST_NETPLAY=1` runs while
  other builds loaded the machine, then passed 3/3 (as did main 3/3) on a quiet one, and alone on both. The
  harness's rounds are deterministic, but ggrs and the resend clock (`rollback.rs` RESEND) run on wall-clock time,
  so a slow round changes the traffic. Not caused by the move; B494 makes it load-proof.
- Timings are from one machine with other sessions on it; medians are stable to ~1 s.
- No gameplay change, so no PCSX2 comparison: the replay and state-hash tests pass unchanged.
