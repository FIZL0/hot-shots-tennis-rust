# M121 — the roster inside the online lobby

Task: one screen instead of the online character select followed by a separate lobby (M120 mockup,
`context/m120/mockup/project/Main.dc.html`: seats on top, this machine's players on a roster card, the others on
compact cards; the settings strip with Start; Leave; the code and Doubles in the top bar).

## What changed

- The online select and the lobby share one layout. `roster::chars` draws the columns on both screens
  (`online_chars`: this machine's players get the full roster column, other machines' and computer players get the
  compact card). `online::lobby::over` then draws the lobby around them:
  - the top bar: Copy link, Public / Paste, the code pill with reach, Singles / Doubles;
  - on the lobby, each seat's focus ring, the host's ✕ and a carried card;
  - the settings strip; Leave, the chat box (it shows the status line too), and Start / Ready.
- `frame::foot` places the strip and bottom row along the view's bottom edge; the columns stop above it
  (`Menu::columns`). `lobby_geo().plates` are the columns' cards, so seats, carrying, kicks and taps still use the
  same `Item`s.
- Screen::Chars and Screen::Lobby are still two states (picking vs. picked) with their old key handling. Moving
  between them only moves the cursor (no slide, `slide.rs`).
- While a local player is picking (Chars), the lobby's own items have no cursor. A tap on the strip, the code, the
  bar or Leave still works (`Act::Lobby` → `lobby_tap` runs the lobby's step for it). Taps on seats, Start or Ready
  don't.
- The court and umpire pickers opened from the lobby draw the lobby (without columns) behind them
  (`online::lobby_behind`).
- The host's ✕ on another machine's seat sits on the figure's top-left corner (`roster::remove_rect`), clear of
  Near/Far and the device pill.
- The LAN host's open compact card shows the LAN join hint (`HOST_LAN`, B526).
- Removed: the court-plate lobby drawing (`lobby::draw`, plates, chips Picking/Waiting/Open).
- `online::sample` now syncs `doubles` and `go` with the sample room, as joining does.

Shots: `context/m121/run3/` (16:9 host, portrait host, 4:3 guest), `run2/` (online select).
Tests: `tools/check.sh -p hst main_menu` 188 passed. `lays_out_at_any_aspect` and `lobby_and_confirm_fit_the_window`
now check against the whole view, with the columns above the strip. The top-bar walk in `cursor_walks_the_grids`
follows the new left-to-right order.

## Not verified / not 1:1

- Start sits in the bottom row (right of the chat box), not beside the strip as in the mockup. Reason: the strip
  needs the full width for five cells at 4:3 and narrower.
- No "Fill with COM" button from the mockup. Reason: computer seats are already added from an open seat's card;
  a second way to do it was out of scope.
- Seats, Start and Ready can't be tapped while this machine's players are still picking. Reason: those are actions
  for after picking (Ready on the select first); keeping them out avoids a half-readied pick.
- The court and umpire pickers' underlay shows the lobby without the columns. Reason: the picker panel covers that
  area anyway.
- In portrait (9:16), compact cards stretch their stat rows to fill the tall column. This is the compact card's
  existing behaviour, not new in M121.
- Not tried between two real machines (only the sample lobby, tests and shots). Reason: unattended run, no second
  peer; net.rs and the wire are unchanged, so the protocol is as before.
- Not compared with the original game: the online lobby is ours, with no counterpart on the disc.
