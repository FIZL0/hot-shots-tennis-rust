# M14 — the legend (FINAL)

## What landed
- `play/main_menu/legend.rs`: `Legend` (groups of items per device), `Last` (keyboard / pad / touch, tracked in
  PreUpdate), glyph art drawn with tiny_skia (keycaps; Xbox A/B/X/Y in colour; PlayStation ✕○□△; Deck dark A/B/X/Y;
  LB/RB or L1/R1; View/Menu or Create/Options pills; d-pad with the used arms lit), one cached image per chip, laid
  out right-aligned on the 640×448 strip y 436..448 spanning the window (`widescreen::Anchor::Span`) at the 4:3
  screen's scale, the row shrunk uniformly to fit. Hidden after a touch (no tap hints) until a key/button.
- `acts(menu, dev)`: each main-menu screen's actions as they stand (row, ready state, pickers, ask pop-ups, online
  pills — `Menu::pill_axes` says which of ↑↓/←→ go anywhere —, Join by Code's keys, lobby buttons via
  `Menu::lobby_ok`). The select and seat assignment show each seated device (plus an unseated last-used one on
  Assign), merged when identical, tagged 1P/2P… when they differ.
- Pause menu (`menu.rs`), its Settings screens (`pause.rs`), result screen (`match_stats.rs`) fill the same legend.
- Keyboard caps follow `Bindings` (`Bindings::key`) for the bound actions plus the fixed Enter / Esc / Q E / Tab.
- `HST_LEGEND=keys|xbox|ps|deck` forces a style for shots.

## Test
`legend_lists_what_each_screen_handles` (main_menu/legend.rs): ~20 start states (every screen, ask pop-ups,
rebinding, pads joined / seated / ready, pickers, host / guest lobby, code / name entry), every state reachable with
the d-pad on that screen; for each driving device it steps each press and compares "handled" (menu differs from an
idle step, or any effect beyond the refusal sound) with the legend: ↕ axis, ↔ axis, ✕, ○, △, L1/R1, □, Select.
It found and fixed: Online's single wide row has no ←→; Join only with a whole short code, Save only with a name,
Clear only with text. 0.07 s.

## Shots
`context/m14/run1/` — 20 menu states (main, settings, graphics, controls, mode, assign, chars, coms, confirm,
courts, online, lobbies, lobby, lobby-ask, lobby-guest, code, name, chars-online, addmods, reloadmods) + pause,
pause ask, pause settings, result stats; each at 1024×768 (4:3), 1280×720 (16:9), 1280×800 (16:10), 1680×720
(21:9), styles rotating keys / Xbox / PS (Deck on five 16:10 ones). Sheets: `context/m14/sheets/` (per screen, and
`strip_*` the legend strips at full res). No overlap with the original's sprites (the result page's own
"✕ Continue / ✥ Score Screen" hints stay above it; the pause art ends above the strip).

## Not verified / not 1:1
- Real pads: vendor → style (0x054c PlayStation, 0x28de / `SteamDeck=1` Deck, else Xbox) and live switching were not
  tried with physical controllers (none attached to an unattended run); shots force the style with `HST_LEGEND`.
- Touch hiding not tried on a phone/touch screen (no touch device here).
- Pause and result legends are hand-listed from their `step` systems (Bevy systems reading input directly, not a
  pure `step`), so the test doesn't probe them; their keyboard caps are literal (those screens use fixed keys).
- The ✕ press on the Mods screens and the main menu's Reload/Later pop-up isn't probed (it reloads the game, opens a
  file picker or reaches the store); taken as handled there.
- Rebinding's Esc cancel is handled outside `Menu::step`; not probed.
- Two-pad per-player groups are covered by the test, not by a shot with two real pads.
- The legend is ours (no original counterpart), so nothing to match 1:1.
