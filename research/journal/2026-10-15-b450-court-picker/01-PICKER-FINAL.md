# B450 — Court picker over the players

Pressing Court on the confirm screen or in the online lobby now opens a panel over the players' area. The screen
stays drawn around it. This replaces B192's separate Select Court screen (`court_screen` is removed).

- `play/main_menu/court_picker.rs`: the rows, layout, step, draw and tests.
  - Row 1 is Random followed by the disc eleven in the original's strip order.
  - Row 2 holds the court mods and appears only when there are any.
  - Each row keeps its own scroll position and its own chosen card (`Menu::court_pick`).
  - ←→ moves one card, ↑↓ switches rows, L1/R1 jumps a page (both stop at the ends), ✕ keeps the pick, ○ restores
    the previous court.
  - A row's head shows "< a-b of N >". The arrows are tappable. Each row has a scroll bar.
  - A card shows the picture, the name, and the surface (for a mod, its game; for Random, "Any of the 11 courts").
  - Pages show 4 to 7 cards depending on the window's width. In tall windows each band is sized to its cards and the
    rows are centred.
  - Quads of the screen underneath that sit inside the panel are dropped, so 100 mods at 4:1 stay inside the menu's
    quad pool.
- Online, the picker shows only courts that every peer has. A new `Hello.courts` field carries each peer's installed
  court mod ids. The menu sends them with `Cmd::Courts` alongside `Cmd::Name`. The host keeps them in
  `Room::courts`, and they follow leave, rejoin and back_to_lobby. `court_mods::values` filters on that map, and a
  vanilla lobby still shows no mods.
- Opened from the lobby, the picker draws over a clone of the lobby with its court card chosen (`Menu::venue_under`).
- Legend: ←→ Choose · ↑↓ Row (only with a mods row) · L1 R1 Scroll (only when the row is longer than a page) · ✕ OK ·
  ○ Cancel. The legend test probes these.
- Screenshots: `HST_MENU=courts`, `courts-mod`, and `lobby-courts` (new: the host's picker, where ACE-42 lacks every
  third court mod). Shots are in `context/b450/run1/` (4:3, 600×1000, 1600×500, lobby).

## Not verified / not 1:1
- Not compared with the concept board ('Concept v4: Court pressed'). The claude.ai artifact isn't reachable from
  here and there is no local copy of LobbyV4.dc.html, so the layout and colours follow the task text.
- B449 (the Lobby v4 / Confirm v4 frame) hasn't landed. The panel covers the current v3 players' area (between the
  title and the bottom bar, left of the side panel, or above it in portrait; `court_picker::area`). When B449 lands,
  `area` needs to point at its court stage, and the v3 help line under the panel still describes the court row.
- In the lobby, a sliver of the seat cards' outer rim shows just below the panel's bottom-left corner. This is
  cosmetic and left as is.
- The interaction across real peers (Hello carrying court ids) is only unit-tested, with no two-machine run. Peers on
  an older build send no ids, so they share no mods. That's moot in practice because the build check refuses them.
- If a guest joins without the host's already-picked court mod, the pick is kept. That guest plays the layout's disc
  court (M8f8's existing fallback); the pick isn't reset.
- Front-end only (ours), so there's no PCSX2 comparison.
