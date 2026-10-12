# M9b native mobile builds — dropped (2026-10-12)

Mid-task the user said: "skip android support, lets get the in-browser build. note that too". M9b is marked blocked
(dropped) in `plan/TODO.md`; the browser build (M3 wasm, M9 touch/PWA) is next.

What was found before stopping (for whoever picks it up again):
- The machine has an Android SDK in `~/Android/Sdk` (build-tools 36, platform android-36, NDK 28.2, emulator, AVD
  `hst34`), and rustup has `aarch64-linux-android` / `x86_64-linux-android`.
- `hst` doesn't build for Android as is: bevy 0.19 needs `android-native-activity` (no Java/gradle) or
  `android-game-activity`, and a cdylib with `#[bevy_main]`. The tried shape: a `crates/hst-android` cdylib whose lib
  path is `../hst/src/main.rs` (kept out of the workspace's `default-members`), `main` split into `start(Cli)`, and
  an `android_main` that opens the first `.iso` in the app's external files dir. Reverted, nothing committed.

## Not verified / not 1:1
- Everything: no APK was built or run (task dropped by the user).
