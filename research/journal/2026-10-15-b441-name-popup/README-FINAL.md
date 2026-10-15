# B441 — Online name uses the Mods page's keyboard popup; mobile keyboard mode

User 2026-10-15: the online name entry should use the same popup as the Mods page instead of its own, and both
should support a mobile keyboard mode.

## What changed
- New `crates/hst/src/play/main_menu/osk.rs`: the on-screen keyboard popup from the Mods page's Browse search (B372)
  moved out of `mods_screen/grid.rs`, so both screens share it: keys, cursor moves (`step` returns what a press does
  to the text, the caller applies it), drawing (`Look`: optional title, last wide key's word, full) and legend.
- Online → Name now opens that popup over the Online menu (its Name row lit, with the title "Your name (the other
  players see it)"). The keys type capitals, Space/Delete/Clear/Save are the wide keys, ○ closes and leaves the name
  as it was, and Save on a blank name does nothing (plays the refuse sound). It still runs as `Screen::Code` with `naming`, so going back,
  pointer rows, `HST_MENU=name` and the phone's page keyboard (`web::code_entry`) work as before. Enter, Start,
  Space, Backspace and the iPhone's Return (it arrives as a "\n" character) all work in it. The relay URL and Join by Code keep the
  Code screen; the old name branches (NAME_KEYS grid, Clear/Save buttons) are removed.
- Mobile keyboard mode (`osk::mobile()`: a touch page/app with no real controller): the popup moves to the top,
  under the touch pad's L1/Select/Start/R1 row. It shows only the field and the wide keys, and the cursor stays on those.
  The device's keyboard comes up under it: in the browser through the page's input (as before), and in the iPhone and
  Android apps through `Window::ime_enabled` (winit 0.30: iOS `becomeFirstResponder`, Android `show_soft_input`).
  On the native apps, tapping the field puts the keyboard down for one frame and brings it back up. `online::touch_typing` is now
  `osk::mobile()`, so the native apps' Join by Code / relay URL use the system keyboard too.
- Tests: `osk::tests::keys_and_moves`; the online name test drives the popup (type, Space, ○ keeps the old name, blank
  Save refused, Save); pointer/legend tests updated for the popup.

Shots (context/b441/): name.png, name-touch.png, name-touch-tall.png (portrait), mods-keys.png,
mods-keys-touch.png. Desktop mods popup unchanged in look.

## Not verified / not 1:1
- Not 1:1: a front-end screen, ours to design (PLAN.md), so there's no original to compare against.
- The native system keyboard (iOS/Android) is untested on a device or emulator. It needs an APK/IPA build and a
  device session. Typed text on Android NativeActivity only arrives if the IME sends key events (winit has no
  InputConnection); if a keyboard doesn't, the grid is hidden in mobile mode there and that keyboard can't type.
  Check this on the next Android/iOS device run.
- The web phone keyboard path wasn't run on a phone: it uses the same `code_entry` input as before (the input covers the
  canvas, so the popup's wide keys can't be tapped there; Return saves, the touch pad's ○ closes). Only checked with `HST_TOUCH=1` shots on the desktop.
- In portrait the mobile popup sits at about 37 % of the height, scaled with the 4:3 menu, not pinned to the top
  of the screen.
