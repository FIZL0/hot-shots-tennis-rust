# Native Android APK — prototype (2026-10-14)

User asked to look into a native Android build again (dropped 2026-10-12, see `2026-10-12-m9b-native-mobile`).

## What works
- `tools/apk.sh [--install]` → `dist/hst.apk`: `hst`'s `src/main.rs` built a second time as a cdylib example
  (`[[example]] hst`, feature `android` = bevy `android-native-activity`), NativeActivity, no Java/gradle: aapt2 link
  (+ `crates/hst/assets` fonts), zip the stripped `.so`s, zipalign, apksigner (debug key). `HST_ABIS="arm64-v8a x86_64"`
  adds the emulator ABI. The example must be named `hst` or the `embedded://hst/…` shaders aren't found.
- `android_main` (main.rs): first `.iso` in `/sdcard/Android/data/io.github.fizl0.hst/files/` (adb push), cwd set there.
- Off Android only: bevy `system_clipboard` (arboard has no Android backend; the in-app clipboard remains) and `rfd`
  (Add mod reports "no file picked"; store downloads are the way in).
- Emulator `hst34` (x86_64, API 34, `-gpu swiftshader_indirect`): Vulkan on SwiftShader, main menu renders,
  keyboard (adb keyevent) navigates.

## Not working yet
- Touch: an `adb shell input tap` did nothing — native touch arrives as bevy `TouchInput`, not the mouse events a
  browser synthesizes; the menu/match touch layer needs to read touches.
- HTTPS (mod store, pkarr relay): `rustls-platform-verifier` panics "Expect rustls-platform-verifier to be
  initialized" — it needs its Java side; use webpki roots on Android instead.
- Gamepads: gilrs has no Android backend ("Gilrs does not support current platform"); winit's Android key events
  carry gamepad buttons, or a gilrs Android backend.
- Getting the ISO onto a phone without adb (Android 11+ hides Android/data from file managers): needs a SAF picker
  (Java/JNI) or reading from /sdcard/Download with storage permission.
- Not run on a real arm64 phone; no match played (SwiftShader too slow to judge performance).
- APK is ~245 MB with both ABIs (~120 MB/ABI stripped).
