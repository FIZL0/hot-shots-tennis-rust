# Not reproducible; most likely a binary from another branch

## What was run (all `RUST_BACKTRACE=1`, ISO symlinked into a scratch dir so the user's settings.txt stays untouched)
- B139's test `open_change_back_in_a_match`: still runs (ISO found, not skipped) and passes.
- `hst ISO --play`, pause, Settings: via `HST_PAUSE`+`HST_PAUSE_SETTINGS`, and via the real path (pause → Down →
  ✕ on a virtual pad, `tools/vpad.py`), toggling Widescreen (settings.txt written). No panic.
- Graphics and Controls sub-screens opened from the pause Settings (`HST_KEYS`): no panic.
- Main menu → match (`HST_MENU=confirm`, `HST_KEYS`), pause → Settings → toggle → Back → resume → pause → Settings
  → Graphics → back → Return to the Menu → menu again: no panic, debug and release.
- With the user's own settings.txt / hints.txt / controls.txt (upscaled textures off → `--no-upscale`): no panic.
- `--play --mod …/fore_pc00_phoebe` (a mod character), pause → Settings → Graphics: no panic.
- `coredumpctl`: no hst core from the user's session.

## Why it probably happened
Parallel slots share one `target/` (`CARGO_TARGET_DIR`). `target/release/hst` changed size twice during this task
(23:23, 23:28) as other slots built their branches. A binary run straight from `target/` can be another task
branch's build, including one cut before B139 (396b13c, 22:26) — whose overlay plugin lacks `init_resource::<Icons>()`,
the exact panic B139 fixed. The user reported it at 22:52 and said "it's working now" while this ran, which fits.
`cargo run -p hst --release` from the main checkout rebuilds main and avoids it.

## B139's test vs the real app
It builds the overlay's plugin with `controls::plugin` in a minimal app and opens `PauseSettings` directly, so it
skips `menu.rs`'s selection step and the rest of `play::plugin` (which needs a renderer). The real-app paths above
were checked by hand instead. Nothing to make it catch: there's no crash in this code to catch.

## Not verified / not 1:1
- The user's exact crash: no backtrace was captured, so the stale-binary cause is inferred, not proven.
- Mouse/touch click on the Settings pill in a full run: not driven (keyboard and pad only; the row hit-test is
  covered by `rows_under_clicks`).
- Not checked against the original: the pause Settings overlay is remaster-only (B126).
