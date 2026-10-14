# B297 — main menu redesign (FINAL)

`play/main_menu/home.rs` (new) draws Screen::Main from `mockups/Main.dc.html` / `Ultrawide.dc.html`:

- Left list: Rematch card (only after a match was started this session), Local, Online, Mods, Settings; a smaller
  Quit pill at the foot (not on the web build). Chosen pill is yellow with `>`; hand cursor as before. The B205
  "Reload" note now sits on the Mods pill.
- Preview panel for the highlighted row: picture (Rematch = the two/four players' portraits; Local = a row of disc
  portraits; Online = court tiles; Mods = installed mods' face icons; Settings = the settings pages), outlined name,
  wrapped description, three fact tiles (Local: singles/doubles/pads; Rematch: players/games/umpire; …).
- Header: "N pad(s) - Disc: SCUS-97610". The help ticker is gone; the B-legend footer stays.
- Rematch (`Menu::rematch`, set by Confirm's Start): launches the same `args` again, re-rolling random court/umpire.
  Local/Online clear it. Test `rematch_replays_the_setup`.
- 21:9: `home::columns(extra)` keeps list and panel at their 16:9 size (panel ≤ 540 units) and adds a
  "New in the store" column (`mods_screen::news`: store entries not installed, unverified ones first) when the
  window has room; `fit_grid` asks for the store listing once. Test `columns_fit_each_aspect`.
- Every back-to-main path lands on its own row (`Menu::main_at(Row)`).

Checked by eye (shots in `context/b297/`): 4:3 (no store column), 16:9, 21:9 at 1680×720 with Rematch and Local
highlighted — layouts match the mockups; text fits. `tools/check.sh -p hst` passes (286).

## Not verified / not 1:1

- No PCSX2 comparison: the front-end is ours to design (PLAN.md), the mockups are the reference.
- Rematch is in-session only (not saved across restarts); pads are session-bound, so a seat whose pad was
  unplugged since launches empty like any missing seat.
- Mockup's "COM #5" rank line isn't in the Rematch card (no rank shown elsewhere in the port to take it from).
- Pictures are 2D portraits/tiles/icons, not the mockup's 3D court flyover / live character (placeholder text in
  the mockup; a 3D preview would need its own render target).
- Store chips use the style colour + the name's initial, not each mod's preview.png (that would cost one download
  per entry on every menu open). The column lists store order, not upload date (the listing has no dates).
- Mod face icons in the Mods preview show only once the character select has baked them earlier in the session.
- The disc font has no `·` or `|`, so the mockup's separators are drawn as `-`.
