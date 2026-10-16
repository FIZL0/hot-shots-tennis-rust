# B512 — no quick-chat in the lobby chat

User (2026-10-16): tapping the lobby's chat box on a phone showed the match's quick-chat line ("R2 next, L2 say:
Nice shot!"), and pads got the R2/L2 phrase inputs (and the R2 L2 Chat legend) in the lobby too.

Cause: the lobby's Chat button asked `chat.rs` for the phrases (`ASK = 2`) whenever the last press came from a pad
(`View::pad`), and on the phones the on-screen pad counts as a pad. `chat::input` read R2/L2 from every pad at all times,
and the menus' legend drew Act::Chat as R2 L2 for pads.

Fix:
- `online.rs`: the Chat button always opens the typed box (`ASK = 1`); `View::pad` is gone (its only use).
- `chat.rs`: `MENUS`, set every frame by `main_menu::online::poll`, swapped by `input`: while the menus are up R2/L2
  do nothing and no phrase shows. In a match (the menu systems aren't running) the phrases work as before.
  A second `ASK = 1` while the box is open closes it, so a pad or a tap can close it without Esc.
- `hst-play legend.rs`: Act::Chat has no pad glyphs and, like Act::Type, no entry for a pad-only group: only T shows.
- Test `chat::tests::no_quick_chat_in_the_menus` (no phrase with MENUS set, a phrase without; the button toggles the
  box); the legend test now checks the empty glyphs / no chip.

## Not verified / not 1:1
- Not tried on a real phone or with a real pad in a live lobby (no online peer in an unattended run); covered by the
  unit test on `chat::input` and the legend's chip test. Front-end/chat are ours, nothing to match in the original.
- With a pad and no keyboard on a desktop, the typed box opens but can't be typed into (no on-screen keyboard for chat);
  the user asked for no phrases in the lobby, so the button now only opens/closes the box there.
