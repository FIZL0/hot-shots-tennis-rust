# B374 — Self-updater for the Linux and Windows builds

Spec: `plan/B374.md`. Built on B317's notice (`update.rs`, the main menu's NEW VERSION OUT card).

## What landed
- `crates/hst/src/update.rs`: a release build carries its tag (`HST_VERSION`, set by `tools/release.sh` through
  `tools/build.sh`, passed into the Steam Runtime container). It asks `releases/latest` at start and every 30 min
  (User-Agent, no auth); `newer()` is semver order (`-alpha.N` < release, numeric parts by number). Errors are only
  logged at info. A build without `HST_VERSION` (local, rolling `alpha`) keeps B317's commit check, never the release one.
- Install: `SHA256SUMS` + `.sig` first (minisign-verify against `PUBLIC_KEY` = the user's `~/.config/hst/release.pub`),
  then the bare binary is streamed to `<exe dir>/hst.new` with progress, sha256 compared, swapped (Linux chmod 755 + rename;
  Windows `hst.exe` → `hst.exe.old`, `hst.new` → `hst.exe`, put back on failure), relaunched with the same args, exit.
  Any failure: `hst.new` removed, old binary kept, the error shown under the card's bar.
- No swap when the exe dir isn't writable or under Flatpak / Steam (`FLATPAK_ID`, `SteamAppId`, `SteamGameId`) or
  there's no asset for the platform: the button is Download (release page).
- Menu: card word Update/Download, panel NOW (our tag) → NEW, progress bar and status; ○ on the card = Later (this run);
  legend shows ✕ Update · ○ Later. Version corner shows the release tag on a release build.
- Settings → Online & Mods → Check for Updates (settings.txt `update_check`, default on; desktop only); `--no-update-check`.
- Lobby: a "different version (host hst X …)" refusal whose X is newer than ours and ≤ the published latest brings
  the offer back and appends "vY is out: update from the main menu." (`online.rs` heard → `update::refused`).
- `tools/release.sh`: refuses without `~/.config/hst/release.key` or minisign; builds with `HST_VERSION=$tag`; uploads
  `hst-linux-x86_64`, `hst-windows-x86_64.exe`, `SHA256SUMS` (also covers the tar.gz/zip), `SHA256SUMS.sig`; verifies
  the signature against `update.rs`'s `PUBLIC_KEY` before uploading.
- Test hooks: `HST_UPDATE_API=<url>` (another releases/latest JSON), `HST_UPDATE_NOW=1` (install on find, no button).

## Verified
- `update::tests` (local HTTP server, throwaway minisign key): newer → swaps (exec bit set), Windows rename path
  leaves `.old`; signature by another key / sums not matching their signature / wrong checksum → old file kept, no
  `hst.new`; same or older version → nothing; no server → error; semver order table.
- By hand, Linux: a debug build with `HST_VERSION=v0.0.1` (and the test key patched in for that build only) in a
  scratch dir, local `python -m http.server` serving a v0.0.2 release JSON + signed sums: the menu shows the Update
  card (`--shot` in `context/B374/offer.png`); with `HST_UPDATE_NOW=1` it downloaded, verified, swapped and relaunched
  the new file with the same args (`<iso> --windowed`), old process exit 0. A tampered binary: "checksum doesn't match
  the signed one", old binary kept, game kept running.
- By hand, Windows under wine: the cross-built `hst.exe` (same setup) became `hst.exe.old`, the new exe took its
  place and was started with the same args; a later start of the real game removed a leftover `hst.exe.old`, and with
  `--no-update-check` made no request.

## Not verified / not 1:1
- Not against a real GitHub draft release: drafts aren't returned by `releases/latest` and their asset URLs need
  auth, and signing a real release needs the user's (passphrase-protected) secret key; the local server stands in.
  First real check: the release after this one lands (`tools/release.sh` with the key).
- Pressing ✕ on the card wasn't driven by input in the e2e run (Hyprland `send_shortcut` keys don't reach the unfocused
  bevy window and I didn't steal focus); `HST_UPDATE_NOW` calls the same `act()`.
- The lobby offer only triggers when net.rs's `BUILD` strings differ; it is still `hst <CARGO_PKG_VERSION>` (0.1.0 for
  every build), so the version tag isn't in the lobby check. Left alone so browser and desktop builds keep playing
  together; it starts working once the crate version is bumped per release.
- Steam / Flatpak detection is by environment variables only; a system-wide install is caught by the write probe.
- Menu layout of the progress bar checked by eye only through code (no shot of a running download: it took < 1 s locally).
