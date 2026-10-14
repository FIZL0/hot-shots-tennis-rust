# B302 — build version and commit on the main menu

- `crates/hst/build.rs` bakes `HST_COMMIT` (`git rev-parse --short=7 HEAD`, `+` when tracked files have
  uncommitted changes, `unknown` with no git), rerunning when the worktree's `HEAD` / `logs/HEAD` change.
  The web build (`tools/web.sh`) goes through the same `cargo build`, so the browser build gets it too
  (`cargo check --target wasm32-unknown-unknown` passes).
- `play/main_menu.rs` `version_corner`: `v<CARGO_PKG_VERSION> - <commit>`, 14 px outlined white, pinned
  (`Anchor::Sides`) to the top-right under the title bar on the Main screen. Not the bottom-right: the key legend
  sits there and hid it. The menu font has no `·`, so the separator is `-`.
- Checked with `tools/shot.sh` at 1280×720 and 600×800 (`context/b302/`): the text stays in the corner at both and
  isn't stretched.

## Not verified / not 1:1
- The original has no version text, so there is nothing to compare against.
- The browser build was only compile-checked, not opened in a browser (it uses the same draw code as native).
