# B208 — Mods page order, Add/Upload greyed on phones

- Installed page rows are now Browse mods (0), Add mod (1), Upload my mod (2), the mods, Reload, Back; the page
  opens on Browse mods. Back from Browse returns to row 0, from Upload to row 2. Help lines, labels, the web file
  input's rect (`add_rect`, now row 1) follow.
- Phone = the web build with `web::touched()` (touch.js: coarse pointer, `?touch`, or any touch) when the page
  opens (`View::phone`, sticky). There rows 1 and 2 draw with `Draw::rows_locked` (B243's greyed pill) and their
  help line says "use the desktop app"; ✕ on one opens a popup (B205's popup style: row name, two lines, an OK pill)
  that ✕/○, a tap on it or a tap outside closes; it eats moves. The file input is hidden on phones.
- Kept the three rows in the same indices on both builds (the task's "below it … at the bottom of the list" read
  as below Browse); rows, hits and tests stay the same shape.
- Shots: `HST_MENU=phonemods` (greyed rows), `phonemods-ask` (popup) added; checked at 16:9, 1600×600 and a
  600×1000 tall window (`context/b208/`).
- Test: `mods_screen::tests::phone_rows_greyed`; `browse_and_upload_pages` opens on Browse.

## Not verified / not 1:1
- Not tried on a real phone or a touch-emulated browser: the web build wasn't run here; phone mode was checked via
  the `phonemods` shots and tests (the flag is the existing `web::touched()`).
- "Upload my mod" overflows its 180-px pill a little in the greyed style (`rows_locked` draws `centred`, not
  squeezed); same as the live style already did. Left as is.
- Menu, not part of the original (front-end is ours).
