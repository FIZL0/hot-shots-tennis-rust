# B531 — mobile chat button: one tap, always up while chat is on

## What was wrong
- **Browser, the on-screen pad's speech bubble.** The keyboard is raised from the tap's `touchend` (`hstChatKeys`, a
  phone opens it only for a focus inside a gesture), but that handler checked `chatShown`. The game reads the tap at
  `touchstart` and, a frame later, opens the box and *hides the bubble* (`BUTTON` was `on && !open && no phrase`). Any
  tap longer than a frame therefore lifted on an already hidden button: no focus, no keyboard, the bubble gone, and the
  invisible full-screen `#codekeys` input took the *second* tap, which finally raised the keyboard. That is both
  symptoms of the report (two taps; the button disappears while you can chat).
- **Browser, the lobby's own Chat button.** Menus act on the finger's lift, a frame after the lift's event: the keyboard
  could never come with that tap either (second tap again).
- **Phone apps.** Nothing raised the system keyboard for chat at all: `osk::system_keyboard` set `ime_enabled` only for
  the menus' text popups / Join by Code (and forced it off otherwise).
- **Browser, upright.** B313's `.touch button.small { bottom … !important }` also caught the chat button, which sat on
  top of Select and Start (tools/web_chat_button.mjs failed its portrait checks on main).

## Changes
- `chat.rs`: `BUTTON` = chat on (in a room, not hidden), the box open or not. The pad's bubble asks with `ASK = 3`:
  closed → open, open → `RAISE` (keyboard back, draft kept). New `OPEN` (box open) and `RAISE` statics. The iPhone
  keyboard's Return arrives as a `"\n"` character: it sends. Test `the_chat_button_stays_and_raises_the_keyboard`;
  the plugin tests share statics, so they're serialised by a mutex.
- `touch_pad.rs` (apps): the bubble stores 3.
- `osk.rs`: the apps' system keyboard is also up while `chat::OPEN` (menus and matches; `Menu` always exists), and
  `chat::RAISE` puts it down for a frame and up again like a tap on the popup's field.
- `web/touch.js`: a touch that *landed* on the bubble raises the keyboard on its lift, whatever the game did in between;
  portrait spot fixed (centred under Select/Start). `tools/web_chat_button.mjs` checks the hide-before-lift case.
- `web/index.html`: `hstChatArm` — a finger landing on the lobby's Chat button (game: `online::chat_arm`, hit-tested
  with `pointer::touch_act`, only with the box closed) makes its `touchend` raise the keyboard; `hstChatKeys` takes the
  input away again after 700 ms if the game didn't open the box (the finger slid off).

## Checks
- `node tools/web_chat_button.mjs context/B531`: all PASS (portrait now too), incl. the new "game took the tap before the
  finger lifted" case. Screenshot of the upright layout looked right.
- `tools/check.sh -p hst-base chat`, `-p hst osk`, `-p hst-play touch_pad`: pass. `cargo check -p hst --target
  wasm32-unknown-unknown --features ship` clean.

## Not verified / not 1:1
- Not tried on a real phone (iOS Safari, Android Chrome, the iPhone/Android apps): no device here. Focus-in-`touchend`
  raising the keyboard is the mechanism B312 already relied on; the apps' keyboard uses the same `ime_enabled` path as
  the name popup (B441).
- The lobby's Chat button path (`hstChatArm`) isn't exercised by a test: it needs the wasm build in a live online lobby.
  A tap shorter than one frame (lift before the game saw the landing) still needs a second tap there.
- Original game: n/a (online chat is ours).
