# B133 — mod character grid with icons

## What changed
- **Icons** (`inspect::face_icon(png, inner)`): cut from the select's baked preview PNGs (B86's cache,
  `cache/previews/<boot>-r4/`, keyed d<char>-<costume> / m-<mod folder>-<newest file time>-<costume>, so an edited
  mod is baked again and its icon follows). No second render path: the baker already draws every disc and mod
  costume with `inspect::spawn_into` once. Head found from the PNG's alpha: body column = median opaque x of the
  upper half (a racket held up/out doesn't pull it), head top = first row with ≥3 opaque px within 20 px of it,
  centre = median x of the 100-px square's middle half; scaled, cut round. Checked on all 105 baked costume-0
  previews (`context/b133/v2.png`); the first try (top row overall) framed Regis's raised racket instead of his head.
- **Select custom page** (`main_menu.rs custom_list`): the list of names is now a grid like the disc grid — one
  framed group (the disc grid's 9-slice box), 4×4 a page on the disc grid's columns, a face square
  ([110,100,90] like the disc's) with the icon and the name under it, each player's coloured hand and "NP" tag on
  their cell exactly as on the disc grid. ←→ one, ↑↓ a row (clamped). Icons are costume 0 (the disc grid shows each
  character's first face); read 2 a frame (`previews.rs icons`), blank until baked. The baker now does every mod's
  costume 0 right after the disc's costume 0, so icons appear early on a first launch.
- **Match panel** (`panel.rs setup`): a `--slot-mod` player's face is its own icon in the disc face's layout —
  measured: INPANE `face_CC_OO.tm2` is a 40×40 round head at the top left of a 64×64 texture, transparent
  elsewhere — so `mod_face` makes the same (40 px). Falls back to the donor's disc face until the preview is baked.
  Disc players keep their disc faces (the original's art).

## Checked
- `--shot`s (context/b133/run2): custom page at 16:9 (p1), 4:3 (p3_43), 2240×960 (p3_wide): grid centred, cards pinned.
- Match `--play --slot-mod 0 fore_pc00_phoebe --slot-mod 1 fore_pc05_brad` vs the same disc match at 3 s
  (context/b133/run3/panels.png): mod faces sit in the ring at the disc faces' size and place.
- Unit test `inspect::face_icon_finds_the_head` (centring, round cut, 40-px panel layout, bad PNG).

## Not verified / not 1:1
- The grid is remaster-only (the original has no mod page); its look follows the disc grid, by eye (menus need
  not be 1:1, PLAN.md).
- Disc characters' rendered icons aren't used: the disc's own faces (select sheet, INPANE) are the original's art
  and stay. The crop works on their PNGs too (v2.png) if one is ever wanted.
- Icon framing is a heuristic on the drawing, not the head bone: a mod whose racket overlaps its head column above
  it, or a very wide hat, can frame off a little (none of the 105 checked did badly; masked heads look small).
- Browser: `mod_face` reads the OPFS cache through the same `read`, not run in a browser build here.
- An unbaked mod (first launch, before the background baker reaches it) shows a blank cell / the donor's panel face.
