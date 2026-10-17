# B544: no pad quick chat; a smaller chat

## Done
- `hst-base/src/chat.rs`: the preset phrases (`PHRASES`), R2 next / L2 say (and the Mute/Unmute phrase rows), the
  phrase timeout (`PICK`), the "R2 next, L2 say" line, `ASK == 2`, B512's `MENUS` guard and its test are gone.
  The typed box (T/Enter, the lobby's Chat button, the touch chat button), `/mute` and `/unmute` stay. `HST_CHAT`
  takes `lines|open` only. No setting brings it back.
- `main_menu/online.rs`: the `MENUS` store is gone; `hst-play/src/play/legend.rs`: comments/test text say a pad has no
  chat key (the strip already showed Chat as T only, unchanged).
- Smaller: the overlay's text 15 → 10 on the 4:3 screen's 448 lines, never under 11 logical px (a phone's), in
  logical px from the window (`min(h, w·¾)`), so it scales uniformly at any aspect; backing padding/radius/gap
  smaller; 4 lines instead of 6; at most 36 % of the width (was 48 %).
- Lobby's chat box (`main_menu/frame.rs` `chat_box`): at most 380 wide and 40 high (the row is 56), centred between
  Leave and Start; its text 14 → 12.
- `web/touch.js`: nothing to change (its chat button only opens the typed box).

## Checked
- `tools/check.sh -p hst-base chat`, `-p hst-play legend`, `-p hst frame`, `-p hst online`: pass.
- Shots `context/b544/run2/`: `HST_MENU=lobby HST_CHAT=open` at 1280×720, 844×390 (phone landscape) and 390×844
  (portrait): the box and lines are smaller, still legible at phone size.

## Not verified / not 1:1
- Ours (chat is an addition): nothing to match against the original.
- No in-match shot: the match uses the same overlay as the lobby shots (same code path, `HST_CHAT` shows it anywhere).
- Not tried on a real phone; the 844×390 window stands in for one.
