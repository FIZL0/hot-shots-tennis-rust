# M101 — Android controllers (2026-10-15)

gilrs has no Android backend, so pads did nothing in the APK. Now:

- **winit patch** (`third_party/winit`, already vendored for M91): on Android, key and motion events whose source has
  the gamepad (0x400) or joystick (0x01000000) bit no longer become `KeyboardInput` / `Touch` (stock winit turned a
  stick's motion into a finger at roughly (0,0)–(1,1) px and pad buttons into unidentified keys). They queue in
  `winit::platform::android::gamepad` (`PadEvent::Key{device, keycode, pressed}` without repeats,
  `PadEvent::Motion{device, axes: [X, Y, Z, RZ, RX, RY, HAT_X, HAT_Y, LTRIGGER, RTRIGGER, BRAKE, GAS]}`), capped at
  1024 events.
- **`crates/hst/src/android_pad.rs`**: drains that queue in PreUpdate (before `InputSystems`) and writes the same raw
  gamepad messages bevy_gilrs writes on desktop, so every menu, seat and match binding works unchanged.
  KEYCODE_BUTTON_A/B/X/Y → South/East/West/North, L1/R1/L2/R2, thumbs, Start/Select/Mode, DPAD_* → d-pad,
  KEYCODE_BACK from a pad → Select; AXIS_X/Y left stick (y flipped), Z/RZ right stick, HAT_X/Y → d-pad buttons,
  max(LTRIGGER, BRAKE) / max(RTRIGGER, GAS) → L2/R2 values.
- **Hot-plug**: a thread lists `InputDevice.getDeviceIds()` over JNI (`jni` 0.21, already in the lock via
  android-activity) once a second; non-virtual gamepad/joystick devices connect with their name and vendor/product
  ids (Sony's → ✕ ○ legend), ones that leave disconnect. Events from an unlisted device connect it at once (`adb shell
  input gamepad …` injects as device −1); events still queued from a pad that has left are dropped (Android sends a
  last motion as a pad goes, which reconnected it as a ghost before that fix).

## Verified (emulator hst34, x86_64, own `-read-only -port 5600` instance, SwiftShader)
- Virtual Bluetooth pad through `adb shell uinput <json>` (vid 054c pid 09cc, bus bluetooth; config types are the
  numbers 100/101/103 = UI_SET_EVBIT/KEYBIT/ABSBIT, strings are rejected): listed as KEYBOARD|GAMEPAD|JOYSTICK →
  "gamepad connected: Wireless Controller (054c:09cc)"; unplug (uinput exits) → disconnected, no ghost; plug again →
  new id connects. Android's DS4 layout maps scancode 304 to BUTTON_X (□) and 305 to BUTTON_A (✕), so positions come
  out right.
- A temporary probe system logged bevy `Gamepad` state: buttons → just_pressed/released, stick → `left_stick()`.
- With ✕ only, the pad went title → menus → "starting the match: … --pads 0,-" (the pad got P1's seat).
- Unit tests: `android_pad::tests` (key map, motion diff).
- Pitfalls: the first menu load on SwiftShader takes ~7.5 min (600 frames) and `mode::hush` drops every press until
  "loading screen down"; frames are ~1 s, so hold buttons >1 s. `-gpu guest` never gets past the splash.
  Headless `screencap` / `emu screenrecord screenshot` are black with swiftshader_indirect, so checks are by logcat.

## Not verified / not 1:1
- **A match played with a pad**: the emulator's SwiftShader died (`DeviceLost`, "driver implementation is at fault")
  loading the match, before any rally. The match reads the same `Gamepad` the menus did, but a real phone + Bluetooth
  pad is needed for the task's done check → plan/HUMAN.md.
- **Real Xbox / PS controllers**: only the uinput DS4 stand-in. Xbox pads' layout (d-pad as hat, View button as
  BUTTON_SELECT or BACK) is mapped from Android's documented key layout files, not seen.
- **Hat d-pad and triggers in the app**: covered by the unit test and the same button path the face buttons proved,
  not watched moving a menu cursor (no screen on the emulator).
- A pad unknown to the device list connects on its first event; that first press is lost (bevy inserts `Gamepad`
  by command, after the frame's raw events). Listed pads (every real one within ~1 s of plugging) are unaffected.
- No rumble (bevy's rumble requests go to gilrs only).

## Verified on a phone (user, 2026-10-15)
The user played on their Android phone with a Bluetooth controller: the controller works (APK built 10:46 from the
M101 tree). Xbox vs PS layout, hat d-pad and triggers weren't reported on separately.
