# B312 — mobile chat button beside Start/Select

The web build's touch Chat was B320's Bevy text label at the chat lines' bottom left, under the menu stick's circle
and nothing like the on-screen pad's buttons. Now it's a button on the page's pad (`web/touch.js`):

- Icon only (an inline SVG speech bubble, `aria-label="Chat"`), on Select/Start's `small` style (same size, border,
  background, opacity, press buzz).
- Landscape: right of Start on their row, the same 0.6u gap as Select–Start. Upright (non-duo) there's no room on
  that row (L1/R1 nearly touch Select/Start already), so it sits centred just under the two.
- Not a pad button: a tap/click counts in `hstChatTaps()`, which `chat.rs` polls (`web::chat_tapped`) and treats like
  T. The game shows it through `hstTouchChat(on)` (`web::touch_chat`, sent on change) while online, chat on, its box
  and the pad phrases closed. Near layer only (a 1v1 on one phone is never online).
- The finger's lift also calls index.html's `hstChatKeys`, which unhides and focuses the code-entry input in that
  same event, so the phone keyboard comes up on the first tap (phones only raise it for a focus inside a gesture;
  before, the box opened and a second tap was needed).
- On wasm the Bevy label is never shown; native touch builds keep it (they have no page pad).

Check: `node tools/web_chat_button.mjs [DIR]` (headless chromium, touch.js alone): hidden until shown, size/style
equal to Start, icon only, placed as above in both orientations, no overlap with any control, a tap counted once,
presses no pad button, raises the keyboard hook, a click counts, hidden counts nothing. 18/18 PASS. Screenshots in
`context/B312/chat-{landscape,portrait}.png`. `tools/web_touch_taps.mjs` and `tools/web_vibrate.mjs` still pass.

## Not verified / not 1:1

- Not run in the full web build in an online lobby on a real phone (needs a built wasm, a disc and a second peer):
  the Rust↔page hooks are compile-checked for wasm32 and the page side is tested alone with the hooks stubbed.
- The first-tap keyboard (`hstChatKeys` focus from `touchend`) is untested on real iOS/Android; headless chromium
  has no on-screen keyboard.
- No original to compare: chat is ours (M20), not in the PS2 game.
