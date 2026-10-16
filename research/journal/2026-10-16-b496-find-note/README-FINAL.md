# B496 — Find warns that listed lobbies can lag or be missing

Find (Screen::Lobbies) builds its list from discovery (LAN + DHT), so a lobby can show up late, linger after it
closes, or differ between players. The page now says so, always, and puts Join by Code one press away.

- A note box under the list (y 346–394 in the 640×448 frame, above the info line), on every state (list,
  searching, none, failed): "Lobbies can take a while to show up, or may be missing." / gold "Join by Code is more
  reliable." Tapping/clicking the note opens Join by Code.
- Rows: lobbies…, Refresh, **Join by Code**, Back. The list shows 6 rows now (was 7) to leave room for the note;
  the empty/searching/failed panel moved down from y 250 to 276 to clear the third button.
- □ / Tab anywhere on the list = Join by Code (legend: "Join by Code"; OK on its row says the same). It opens the
  same code entry as Online's Join by Code (`net.go` cleared); Back from it lands on Online's Join by Code row.
- Test: `online::tests::find_offers_join_by_code` (row OK, □ from a lobby row and from Refresh, Back row still last).
  The legend probe test covers the new □ entry.
- Shots (`HST_MENU=lobbies|lobbies-searching|lobbies-failed`, 1280×720, 1400×600, phone 420×860 `lobbies-none`)
  in `context/b496/`: everything fits, scaled uniformly.

## Not verified / not 1:1
- None needed against the original: a front-end screen of ours (no Find in the PS2 game).
- Not tried on a real phone/touch device; the portrait shot lays out fine and the row/note are hit targets.

## Side fix: `mods_screen::tests::scrolls_and_taps` race
With this change the full `hst` suite failed `scrolls_and_taps` every run (it passed alone, and HEAD passed): the
Installed grid's "Add mod" place (`grid::ADD_AT`) was one process-wide static that every test drawing the Mods page
resets, so the new timing let another test clear it between this test's draw and its read. The app keeps the
shared Mutex; under `cfg(test)` each thread has its own (`grid::add_at`/`set_add_at`). Full suite: 172 passed.
