# M102 — Android touch controls

Native Android touches already reached Bevy as `TouchInput` (winit's Android backend), and the menus' tap handling
(B221) reads `Touches`, so menu taps worked once the app ran. What was missing was the on-screen pad: `touch_pad::on()`
only switched it on for iOS. Done:

- `play/touch_pad.rs`: on for `target_os = "android"` too, gamepad named "Touch controls (Android)"; safe area from
  the activity's window insets (`android.rs`, JNI: `getRootWindowInsets` → `getInsets(systemBars | displayCutout)` on
  SDK ≥ 30, the `getSystemWindowInset*` getters before), polled every 30 frames like iOS's, divided by the window's
  scale factor; the press tick is `View.performHapticFeedback(VIRTUAL_KEY)` (honours the phone's touch-feedback
  setting).
- **Bug found on the way, iOS too**: phones default to Aspect Ratio 16:9 and `hud_gamma` draws the UI into the
  centred 16:9 view, so the pad (laid out in window coordinates, which include the bars) was cut off / shifted on a
  20:9 phone. The pad now has its own UI camera (`PadCamera`, order 60, transparent, premultiplied blend over the
  final window, HDR like the others so the bars don't go grey) and targets it with `UiTargetCamera`; active only while
  the pad shows and no loading screen is up. Side effects: the pad sits over the chat and stays on the pause screen
  (it used to be hidden there with the rest of the HUD, but the pause menu is played with it).
- `graphics.rs` MSAA: the Phone preset's 2× isn't supported everywhere (SwiftShader: "Sample count 2 is not
  supported"); if the adapter can't do 2 samples on Rgba16Float/Depth32Float it uses 4×, as the browser path did.
- Settings → Vibration says "haptic" on Android as on iOS.

## Checked on the emulator (`hst34`, API 34, x86_64, SwiftShader, 2400×1080 landscape)

- Window 2272×1080 px with the cutout letterboxed on the left; the logged safe area was [0, 0, 48, 0] logical
  (nav bar on the right) in an 865.5×411.4 window: the face buttons sit left of the nav bar.
- Menus by touch: tapping a menu row opens it; pad ○ goes back, ✕ selects; the corner stick (dragged with
  `input motionevent`) moves the cursor; a character portrait tap picks; ✕ on the confirm screen starts the match;
  the pad is hidden under the loading screen.
- Match by touch: the stick appears where the left thumb lands and follows the drag; **multi-touch** (raw two-finger
  `sendevent` on `/dev/input/event2` as root: stick held up + ✕) lit ✕ while the stick was held and the player swung
  at the serve; START (tap) opens the pause menu, ✕ tap on it resumes.
- Raw injection notes: `adb root` first; the touchscreen is `virtio_input_multi_touch_1` (device 4 in `dumpsys
  input`, the only one bound to display 0), axes 0..32767 over the natural portrait 1080×2400; for the landscape
  (rotation 90) screen point (lx, ly): raw x = (1080 − ly)/1080·32767, raw y = lx/2400·32767. `dumpsys input`
  RecentQueue shows what the dispatcher sent (Last Raw Touch stays 0 for injected events).
- Once, the first two-finger run left ✕ drawn lit on the pause screen for ~25 s; a later ✕ tap still resumed the
  match (a stuck press would have swallowed it) and a repeat run released cleanly, so it looks like SwiftShader's
  multi-second match frames, not a lost touch end. Not reproduced.
- `tools/check.sh -p hst`: 420 passed (incl. `a_gamepad_the_menus_dont_see_and_a_real_pad_replaces`).

## Not verified / not 1:1

- Not run on a real Android phone (none here): emulator only, x86_64; arm64 build not run.
- Haptic tick not felt (emulator has no vibrator); the JNI call ran without errors.
- Edge-to-edge cutout insets (Android 15+, where windows draw under the cutout) not exercised: the emulator is API 34
  and letterboxes the cutout, so only the nav-bar inset was seen.
- Full match play limited by SwiftShader speed (match loading 496 s, frames of seconds; with default shadows on and
  MSAA off the first load died with a SwiftShader `DeviceLost` in clustering — emulator driver, not ours). A couple
  of points' worth of input checked, not a whole match.
- iOS: the pad-camera change is shared code but not built or run on an iPhone here.
- The one-off lit ✕ above (not reproduced).
- Front-end/pad layout is ours (not 1:1 by design).
