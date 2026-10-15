# M99: Android start page when there's no disc (2026-10-15)

The user installed the APK on a phone without adb, so the prototype's only way in (adb push the ISO into
`/sdcard/Android/data/io.github.fizl0.hst/files/`, hidden from file managers since Android 11) was closed to them.

## What changed
- `android_main` (main.rs): `disc.pack` in the app's external files dir, else the first `.iso` there (the adb route
  still works), else `android::no_disc`.
- `crates/hst/src/android.rs`: the start page (Bevy UI in vmin units, so it fits any aspect ratio). It names the disc
  (Hot Shots Tennis (USA), SCUS-97610, v1.00) and has a "Choose disc image" button. A tap anywhere, a click or any key
  or pad button opens the picker. The picked document is imported on a thread with `Iso::write_pack` (the same pack
  and disc-ID check as the desktop's M13 import), with a progress %. Errors (not a disc image / wrong disc / copy
  failure) show on the page, and a tap tries again. When the import is done, the app restarts.
- `crates/hst/android/io/github/fizl0/hst/HstActivity.java`: a `NativeActivity` subclass. `pick()` opens
  `ACTION_OPEN_DOCUMENT` (SAF: Downloads, SD cards, Drive…, no storage permission needed). `onActivityResult`
  detaches the picked document's fd for the native side to read through `taken()`. `restart()` starts `Restart`
  (its own `:restart` process), which kills the game's process and launches it again, because winit's event loop can
  only be made once per process. Rust calls these through `jni` 0.21 (already in the lock via cpal).
- `tools/apk.sh`: javac `--release 17` + d8 `--min-api 26` → `classes.dex` in the APK, `hasCode=true`, the
  activity is `HstActivity`, plus the `Restart` activity. Still no gradle.

## Verified (emulator hst34, API 34, x86_64; own read-only instance, `-port 5620`)
- Fresh install with no disc: the start page shows (landscape 2400×1080). A tap opens DocumentsUI. Picking
  Downloads/Hot Shots Tennis (USA).iso imports `disc.pack` in a few seconds (170820213 bytes, the same size as the
  desktop pack; hash d89b5d230259faef). The app restarts by itself (new pid), and the new process loads the main
  menu from the pack (logcat: controls.txt written, "0 custom characters", bounds).
- Enter (keyevent) opens the picker too. Picking a text file shows "That file isn't a disc image…" and the page
  stays usable.

## Not verified / not 1:1
- The main menu is black on this emulator after the restart, and also on a plain launch with the pack or with an
  adb-pushed ISO (no M99 code involved). Frames do reach SurfaceFlinger (~1/s) and the start page itself draws, so
  it's in the game's renderer on SwiftShader, or a regression since the 2026-10-14 prototype. Logged as **M104**.
  So "reaches the main menu" is proved by the logs, not by a picture.
- Not run on a real phone (none attached). arm64 not built this time (only the x86_64 emulator ABI).
- The import keeps the whole pack in the app's external files dir. Uninstalling the app deletes it (Android's rule).
- If Android kills the process while the picker is open (low memory), the pick is lost and the page asks again.
- Front-end is ours (PLAN.md): no comparison with the original needed.

Note: at first my emulator didn't start (the AVD was locked by another agent's `-read-only` instance), and my adb
commands went to that agent's emulator (5584): an uninstall/reinstall of the app and an ISO push, which I removed
again. From then on I used my own `-read-only -port 5620` instance. Parallel agents should always pass
`-read-only -port N` and `adb -s`.
