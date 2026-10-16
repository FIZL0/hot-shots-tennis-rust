# B447 — long single-line texts slide sideways instead of overflowing (redo on B448's layout)

User 2026-10-15: the Mods panel's "Uploaded: <url>" line and the upload status's discussion URL ran past the panel.
The first take (wip/B447, e8c4703d, journal `2026-10-15-b447-long-line-marquee`) changed the old side panel, which
B448 replaced with the roster-look grid and detail column; this redoes it on that layout.

## What changed (`mods_screen/grid.rs`)
- `marquee` (B444's sliding line: rest 1.2 s, 30 px/s, rest, start over, clipped to its box; clock = seconds the cursor
  has stayed put, `Browse::roll`) draws nothing when its box has no room.
- New `para_roll`: `look::para`'s wrap (same line cap), but a line still wider than the box (one long word: URL,
  path, file name) slides with `marquee` instead of ending in "...".
- The chosen mod's panel: its facts (`Uploaded: https://…/discussions/12`, `Folder <path>`, `From <game>`) and the
  grey uploader/costumes/hand line slide; `detail_text`'s about/note and an open question's words use `para_roll`.
- The footer status (the upload's end: "Allan is up (23 MB): … <url>") slides in its room instead of ending in "...".
- `HST_MENU=uploadmods HST_CHILD=done [HST_LONG=<secs>]` (mods_screen.rs `sample`): that status, the first mod
  hovered, long lines slid `secs`. Shots `context/b447/run3/up-0.png`, `up-4.png` (16:9), `up-sq.png` (900×900):
  link at rest, then slid to its end, inside the panel and inside the footer at both aspects.
- Test `a_long_link_stays_in_its_panel_and_slides` (kept from the wip): no glyph past the box, rests at start, a
  full box of it partway, last glyph at the right edge at the end.

## Not verified / not 1:1
- There's no original to compare with: the Mods page is ours (PLAN says the front end doesn't have to match 1:1).
- It only slides by itself. You can't drag, scroll or use the stick to move along a long line (the task allows
  either). The slide needs no hover, so it works on touch too.
- The clock restarts only when the cursor moves, not when the status changes. A new footer status picks up partway
  through the slide's cycle, then keeps cycling.
- Other menus' `look::para` / `options::para` paragraphs are unchanged. They still cut a word that's too long with
  "..." (it stays inside the box, it doesn't spill out); none of them is a URL that I know of.
- Mod names on tiles and the panel's name keep B444's behaviour (they shrink, then slide).
