# M91a1 — native iOS touch controls

The iPhone app (M91) only took a paired controller. This adds the browser's on-screen pad (`web/touch.js`, B247)
natively: `crates/hst/src/play/touch_pad.rs`, added to the base app (`main.rs`) so it lasts through mode switches.

## How
- **A virtual gamepad** named `Touch controls (iPhone)` (so `emote::touch` gives L1/R1 their emotes), vendor id Sony
  (the legend draws ✕ ○ □ △, the same as the buttons). It connects through `GamepadConnectionEvent`, and `drive` (PreUpdate,
  after `InputSystems`, before `mode::hush`) sets its buttons and left stick on the `Gamepad` directly. Menus and
  matches read it like any pad.
- **Shown while no real controller is connected.** When a real pad's `Gamepad` shows up, ours is removed in that same
  PreUpdate, before Update's slots and seats see both pads (B247's rule).
- **Touches**: read from `Touches` after `InputSystems`. A touch that lands on a control keeps that control until it
  lifts, and its `just_pressed` is cleared, so the menus' pointer (`pointer.rs`), pause and result taps and the chat tap
  never see it. Each finger counts separately, and a button stays down for at least one frame (B165/B274). A finger that
  disappears without an end lets its control go.
- **Layout**: `web/touch.js`'s landscape CSS, with u = min(7 % w, 12.5 % h). The face diamond and the shoulders and
  Select/Start sit where the CSS puts them, and each hit area reaches 0.2 u past the button's edge. The stick has a
  2.6 u base and a 1.2 u knob, and reaches full tilt at the base radius. In menus (no `Game`, the pause screen, or the
  result screen) it is a fixed circle at the bottom left. In a match it appears where the thumb lands in the left 45 %,
  below the top 18 %. Insets come from the UIView's `safeAreaInsets` (Dynamic Island side, home indicator), read on
  the main thread every 30 frames (`ios::safe_area`).
- **Art**: Bevy UI nodes with the CSS colours (#0004 wash, #fff5 outline, #fff6 when pressed). ✕ ○ □ △ are drawn with
  tiny_skia, and the small buttons' words use the menus' bold font, at 70 % opacity as on the web.
- **Haptics**: a press gives `UIImpactFeedbackGenerator` (light) on the main queue, gated by Settings → Mobile → Vibration
  (`touch_pad::BUZZ`, set from `main_menu::step`). The iOS row now has its own text.
- **The 1P seat is the screen's**: `touch_page()` is true natively when `touch_pad::on()` is true (iOS, or `HST_TOUCH=1`
  anywhere). As on a touch browser, that means a Touch tile, the Mobile settings tab, a real pad's first press taking 1P,
  16:9 default aspect and fewer downloads at once. `PadSlots::update` lets the keyboard seat (the phone's) take the
  touch pad in a match, as in the browser (B244).

## Checks
- Unit tests (`touch_pad::tests`): layout vs the CSS (no overlapping hit areas, everything inside the safe area), taps
  shorter than a frame, two fingers on one button, stick tilt and clamp, menu corner vs match zone. An App test checks
  that the pad connects under its name, that a tap on ✕ reaches the Gamepad and not `Touches`, that a tap elsewhere does
  reach `Touches`, that the menu stick drives `left_stick`, and that a real pad connecting removes the touch pad in the
  same frame.
- Desktop `HST_TOUCH=1` shots: `context/m91a1/menu.png` (the corner stick at rest, the diamond, the top row) and
  `match.png` (the stick hidden until touched). The layout matches the browser's.
- `tools/ios-build.sh check -p hst` is clean. An IPA is built: `context/m91a1/HotShotsTennis-m91a1.ipa`.

## Not verified / not 1:1
- **Not run on the phone**: no device here. Real multi-touch, the safe-area read, the haptic tick and the lifecycle
  (suspend/resume with fingers down) are untested. **Waiting on the user**: sign and install the IPA, then check that
  the menus can be tapped and driven by the pad and that a match plays, with a controller connected and without one.
- **Portrait layout** (web B112/B313) not ported: the IPA is landscape-only (Info.plist).
- **Chat button** (web B312) not added: the native chat already opens from its own tap target (`chat.rs` `Tap`).
- **The browser's page work** (audio unlock, fullscreen, edge-swipe and selection guards) doesn't apply natively.
- **Touch pad shown as "Pad 1"** on the Local tiles (the browser has the same issue): new task B389.
- **No PINE or oracle proof**: this is input only and doesn't touch the sim. Its presses go through the same gamepad
  bindings as a real pad.
