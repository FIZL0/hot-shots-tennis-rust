# B492 — release hardening

## Audit

- **Cargo profiles**: there was no `[profile.release]`, so the stock one applied (no debug assertions, no overflow
  checks, no debug info). Now spelled out in `Cargo.toml` with a comment, so nobody flips it for a test.
  No `dynamic_linking`, no `bevy_dev_tools`/`bevy_remote` anywhere in the graph (`cargo tree -i`).
- **Bevy features**: `debug` and `bevy_debug_stepping` stay: `mode.rs` needs them at run time (resource type
  names; a mode's start a slice a frame, P50). Not dev features in our use.
- **HST_\* variables**: ~90 runtime ones (fixtures for menus/popups, stress runs, census, net lag/desync/trace,
  key scripts, shade dump, HST_BG, HST_WINDOW…). They can't be gated on `debug_assertions`: tools/shot.sh, the
  benches and court_previews run `cargo build --release` binaries with them. New hst feature **`ship`**: the player
  builds (tools/build.sh linux+windows, apk.sh, ios-ipa.sh, web.sh, pages.yml beta linux+windows) build with it, and
  `main.rs` `dev_env` removes every `HST_*` from the process environment first thing in `main`/`android_main`
  (before any thread), except `HST_FPS` (frame log, the web's `?fps` twin), `HST_TRACE` (menu input trace, meant for
  bugs on a player's own desktop) and `HST_TEXTURE_RELOAD` (documented for texture authors). One place instead of
  ~100 call sites across files other sessions edit. Compile-time ones (`HST_VERSION`, `HST_BUILD`, `HST_SITE`,
  `HST_COMMIT`) are untouched. wasm has no process env.
- **Log levels**: `ship` also turns on `log`/`tracing` `release_max_level_info`: debug/trace callsites (ours, bevy's,
  wgpu's) compile out of player builds; the runtime filter was already bevy's `info,wgpu=error,naga=warn`.
- **Debug keys / overlays**: none bound by default; `HST_KEYS` (scripted keys), census plugins and `HST_PASSES`
  are env-gated, so `ship` drops them. `--inspect`/`--character` viewers are CLI flags, kept (mod authors use them).
- **Unwraps on user data**: CLI parsing falls back on bad values; `--dump-textures` panicked on an unreadable ISO →
  now an error message and exit 2. settings.txt, names, controls parsing index known positions only; the `expect`s
  on reads are on embedded fonts/skeleton. Mod loading is fuzzed already (`HST_FUZZ` test in mods.rs).
- **tools/release.sh**: refuses to run if build.sh/apk.sh/ios-ipa.sh lost `--features ship`.

## Verified

- `cargo build --release -p hst --features ship`, then tools/shot.sh (`--stage 1 --play --shot-at 8`) with
  `HST_FPS=1 HST_PASSES=1`: frame-log lines present, 0 `passes … ms` lines over 9.4 s.
  Same run on the plain release build: 2 `passes` lines (control). Logs in `context/b492/`.
- `cargo tree -e features -i tracing`: `release_max_level_info` only with `--features ship`.

## Not verified / not 1:1

- Not a gameplay change (no oracle/PINE needed); the original game isn't involved.
- The Windows, Android, iOS, web and CI (pages.yml) builds with `ship` weren't built here (same feature, same code
  path as the Linux build that was); the docker Steam Runtime build neither.
- The unwrap audit was a spot check of the user-data entry points (CLI, settings, names, controls, file reads), not a
  line-by-line read of every `unwrap` in the ~100k lines.
