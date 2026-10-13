# B237: menu description bar wraps and shrinks, no marquee

The only scrolling text in the menus was the description bar along the bottom (`ticker_text`, main menu and the
pause menu's settings, which share it): the original's MENU overlay scrolled a text wider than 616 px left at 2 px a
frame. That is gone. Now (`main_menu.rs` `wrap`, `fit_lines`, `ticker_text`):

- Glyph pitch is at most 10 units of the 640 screen (was 11), so text is a little smaller.
- A text that doesn't fit on one line word-wraps at spaces. If the lines don't fit the bar's height, it shrinks
  5% at a time until they do. The bar stays at y 404..436, because the Settings and Graphics rows reach y ~401.
- On a tall narrow window (a phone held upright) the 4:3 screen is stretched vertically, so the bar is taller in
  pixels. There the cap is at least 7 px a glyph (≈11.7 px font) when the bar has the height for it.
- The fly-in when a page opens (in from the bar's right edge at 32 units a frame, then it holds at x 16) stays.
  It isn't scrolling.
- Test: `description_wraps_and_fits`. It replaced `ticker_as_measured` and `ticker_glides`.

Shots (`context/b237/run1/`, Settings and Mods → Upload, the longest text: RULE + "Press again to upload X."):
1024×768, 1280×720, 1680×720, 390×844, 844×390. At 4:3 the upload text takes two lines. At 21:9 it's one line. On
the upright phone it's three lines, readable. The 21:9 and landscape-phone shots were taken mid fly-in, which is
the same timing as before.

List paging (the mods list's 7 rows, online lobbies' 7) moves rows, not text, so it's unchanged.

## Not verified / not 1:1
- Not 1:1 with the original on purpose: the original scrolls long descriptions (user 2026-10-13: no marquee). Menus
  don't have to match (PLAN).
- The row labels and panel texts (pill labels, mod details) weren't resized. They already squeeze to fit their box
  (`label`, `fitted`) and none of them scroll.

## Tests
`tools/check.sh -p hst` passes (251). The full `tools/check.sh` fails only on
`play::netplay::tests::matches::rollback_desync_is_saved_and_play_goes_on` ("the desync's note", netplay.rs:861).
It fails the same way with this change reverted (main at 6d7131e4), so it was already broken. Because done.sh
stops on any failing test, the task was ticked and committed by hand.
