# B517 — mobile chat button stays up while typing

**Cause.** `chat.rs` showed the touch chat button only while the box was closed (`!chat.open`), so a tap hid it.
In the iPhone/Android apps nothing raised the system keyboard for chat: `phone()` only drives the browser's input,
and `osk.rs`'s `system_keyboard` (the owner of `Window::ime_enabled`) only wanted it for the menus' text popups and
Join by Code. The "Say:" field sat at the bottom left, under where a phone keyboard comes up.

**Fix** (`hst-base/src/chat.rs`, `hst/src/play/main_menu/osk.rs`):
- `BUTTON` = chat on and no pad phrase up. The button stays up while typing, in the browser (`hstTouchChat`) and on the
  native touch pad. A tap with the box open sets `chat::RAISE`. The native keyboard goes down for a frame and comes
  back up, like a tap on the osk field. In the browser, the button's own touchend already re-focuses the input
  (`hstChatKeys`).
- `chat::TYPING` (box open) makes `system_keyboard` enable the IME on iOS/Android, so the system keyboard comes up.
  It goes down once the line is sent or the box closes.
- The iPhone keyboard's Return comes as a `"\n"` character and now sends the line, as it does for the name and the code.
- `chat::TOUCH` (set from `osk::mobile()`): while typing by touch, the chat column moves to `top: 16%`, just under the
  on-screen pad's top row and clear of the keyboard.
- Test `chat::tests::the_button_stays_while_typing`. The chat tests now share a lock, because they share statics.

Shot: `HST_TOUCH=1 HST_MENU=lobby HST_CHAT=open tools/shot.sh context/b517/open.png`. The bubble stays beside
Select, and the field and lines are in the top half.

## Not verified / not 1:1
- Not tried on a real iPhone/Android device or in a phone browser (none here). That the IME raises the keyboard
  and sends `KeyboardInput` rests on the existing name/code entry, which already works this way.
- 1:1 doesn't apply: chat is our own addition, with no original to compare against.
