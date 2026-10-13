# B214 — Online menus as a pill grid

- Online screen: Quickplay wide on top (as the confirm screen's Start), then Host/Find, Join by Code/Name,
  Relay/Back in two columns (`ROW_RECTS`). ←→ now move the cursor; Relay changes on OK only.
- Lobby: the host's buttons three to a row under the seats (Start, Characters, Court / Copy Link, Leave), the
  guest's Ready, Characters, Leave in one row (`Menu::lobby_rects`). The guests' seats stay a column above them; ↓
  from the last seat goes to the buttons. Vote is out of the grid (B226 removes voting; the vote panels' code stays
  for B226 to delete).
- One rect list per screen drives drawing, ←→↑↓ (`pill_step`: nearest pill that way, ←→ along the row, no wrap) and
  taps (`pill_at`; the Online screen now takes clicks/taps too).
- Test `cursor_walks_the_grids` walks both grids (and a guest's row); existing lobby tests updated for the new rects.
- Shots (`context/b214/`, HST_MENU online / lobby / lobby-guest at 960×720, 1280×720, 1680×720): the grids lay out
  on the centred 4:3 screen at every aspect, nothing clipped.

## Not verified / not 1:1
- Not compared to the original: the online menus are ours (no counterpart in the game; front-end isn't 1:1 per PLAN).
- The Lobbies list (Find's results) stays a list: it's a scrolling list of lobbies, not buttons.
