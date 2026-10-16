# P61 — split the menus off the match (FINAL)

Goal (after P60's hst-base): the match core as its own crate under the menus, so a menu edit rebuilds only `hst`
and a match edit can be checked with the match's own tests. Pure move, no behaviour change.

## What moved

New workspace crate `crates/hst-play` (~27k lines): `play.rs` and every `play/` module except the front-end
(`main_menu.rs`, `main_menu/`, `menu.rs`), plus `mode.rs`, `preload.rs` and a new `stage.rs` cut from `main.rs`
(court_layout, models, for_models, mesh, image, orbit, `Sky`/`SKY`, point_clouds). `hst` (~27k lines) keeps
`main.rs`, `loading.rs`, `mash.rs`, `sandbox.rs`, `update.rs` and the front-end. Its `main.rs` re-exports the
match (`pub use hst_play::*`, which itself re-exports hst-base) and splices the front-end into one `crate::play`:

    mod play { pub use hst_play::play::*; pub mod main_menu; pub mod menu; }

so every `crate::play::…`, `super::super::…` and `crate::mode` path in the menus still resolves; menus files
didn't move. Items the menus use from the match became `pub` (fields of `Game`, `Pads`/`SlotPad`, `Player`, panel
art, menu_art, emote, touch_pad, mode's `Loading`/`hush`/`busy`, …). `mode.rs`'s own-crate list is now
`hst::`, `hst_base::`, `hst_play::`. The fonts stay in `crates/hst/assets/` (`tools/apk.sh` packages that dir);
`menu_art.rs` includes them by relative path.

## How the edges were cut (commit 1/2, inside one crate first)

- pause state + preview faces: `play/front_end.rs` `FrontEnd { paused, set_paused, face }`, a resource the pause
  screen's plugin (`menu.rs`) inserts; core calls `front_end::paused/set_paused` (no resource = never paused, the
  disc's own faces). The match-start sound hook moved from `menu::plugin` to `front_end::plugin`; `main.rs`
  `add_match` adds `play::menu::plugin` beside `play::plugin`. Test `the_match_pauses_through_the_front_end`.
- `OnlineNotice`, `CHOICES` → `netplay.rs` (pause.rs re-exports them; recover uses netplay's).
- word.tm2 labels (`FONT`, glyphs, sheet, label, shrink) → `play/word_label.rs` (intro, menu_art, pause, main_menu
  use it).
- the key legend's types and drawing → `play/legend.rs`; the menus' rows (`acts`, `groups`, …) stay in
  `main_menu/legend.rs`, which re-exports the core part.
- `screen_point`, `pointer_press` → hst-base `widescreen.rs` (test `window_to_screen` moved with them).
- `netplay::mode` takes the match `Mode` (callers build it with `match_mode`) instead of calling the menu.

## Measurements

Edit-one-file cycle like P60: change one used string literal (an `info!` text) in the file, `cargo test -p <crate>
--no-run -j 5`, before (e36edd28, own worktree + target) and after interleaved, six runs each. The box was busy
(load 15–32 from the other slots), so the numbers are noisy; medians:

| edited file | before (s) | after (s) |
|---|---|---|
| `play.rs`, `-p hst` | 11.4 18.7 12.5 9.6 10.9 10.6 → 11.2 | 8.1 11.9 11.0 6.7 8.0 10.6 → 9.4 |
| `play.rs`, `-p hst-play` (new) | — | 7.8 6.4 8.3 6.5 5.6 6.1 → 6.5 |
| `play/main_menu.rs` | 15.7 12.5 36.9 13.3 11.6 11.9 → 12.9 | 6.5 9.0 6.5 8.1 8.7 9.1 → 8.4 |
| `net.rs` (hst-base), `-p hst` | 29.0 26.3 19.8 22.0 19.4 25.0 → 23.5 | 20.6 17.7 24.2 24.9 21.8 21.5 → 21.7 |

So a menu edit ~13 → 8 s, a match edit ~11 → 9 s through `hst` or ~6.5 s checked with hst-play's own tests, a
base edit about the same (it now rebuilds two crates above it instead of one bigger one). (An unused `const`
appended to the file was tried first: ~3 s everywhere, incremental skips it, not comparable.)

## Tests

Same 490 tests, 491 with the new hook test (`--list`: before hst 342 + hst-base 148; after hst 167 + hst-play 174 +
hst-base 149). Passed: `check.sh -p hst-play` (145, netplay matches skipped), `-p hst` (167),
`HST_NETPLAY=1 -p hst-play netplay` (25 of 26 under load; `rollback_time_sync_spreads_the_waits` failed once
"host at tick 866, guest at 863" and passed twice alone: a real-time test, load-flaky like B494), the ignored
`synctest_through_a_change_of_ends`, `-p hst-base window_to_screen`. wasm32 release build: 17 exports in the JS
(16 web.rs + `run`), as before. Android `cargo check --example hst --features android --target
aarch64-linux-android` clean (needs the NDK's CC/linker env from `tools/apk.sh`). Release game run: `--shot`s of
the main menu and a `--play --stage 1` match draw as before (court, sky, HUD, legend). No new warnings.

## Merge note

Other agents' branches that edit a moved file (`crates/hst/src/play.rs`, `play/*` outside the front-end,
`mode.rs`, `preload.rs`) or the stage code in `main.rs` land on the new paths; a new file they add under
`crates/hst/src/play/` belongs in `crates/hst-play/src/play/` unless it's a menu. Paths in `plan/TODO.md`, `C2.md`,
`P12b.md` were updated; AGENT.md's netplay test command is now `-p hst-play`; PLAN.md's code map has the new crate.

## Not verified / not 1:1

- No PCSX2 check: no gameplay change (pure move), so nothing to compare with the original.
- iOS not compiled (no iOS toolchain on this box); the iOS glue is in hst-base, untouched by this move.
- The pause screen wasn't opened in a running game (a `--shot` can't press Esc in an unfocused window); covered by
  the new hook test and the existing pause/menu tests.
- Timings measured under heavy load from the other slots: noisy, medians only roughly comparable to P60's.
