# B541 — custom names in the original's seat-tag style, 6-character limit

## What the original's tags are (from the disc, `context/b541/`)

- Over the head: `AZUMA/C_EFF/EFFCT.XB0` `panel/i_playerinfo.tm2`, 4-bit, five 40×32 cells (1P–4P, COM).
  White words (flat, no fade) leaning right ~0.27 px/px, a grey (64) ~2-px outline, words' ink rows 1–18; the
  pointer under them: white triangle 14 px across at row 22 to a tip at row 29, same outline (rows 20–31).
  Tinted with the player's panel colour (the marker material's base colour).
- Point boards / final board: `INPANE.XB0` `inpane_p.tm2` 40-px cells (src rect 40×24 from y 0): upright white words,
  near-black (30) 2-px outline, ink rows 4–22 (19 px), from x 5.
- Stats page: `result_status01` rows at y 392 + 24·slot, 48×24: same look as `inpane_p` (30 outline, 19 px).
- The disc has only these words in that lettering (no alphabet in it anywhere: `word.tm2` is a plain menu font,
  `NameEntry_00` only button words), so a name is set in the remaster's menu-art font (M PLUS Rounded ExtraBold,
  `menu_art::HEAVY`, the one the remastered result sheets already re-set "1P"/"COM" in) with each tag's colours,
  outline width, slant and height, via `menu_art::stamp`.

## What landed

- `hst-play/src/play/name_tags.rs`: `Style::{Head, Plate}`, `tag_size` / `tag_image` (rendered at `menu_art::K`×,
  sharp). Over the head the B371 UI tag (Bevy text in a coloured bubble) is replaced by a camera-facing quad exactly
  like the marker: same anchor, size law (`markers::half_width`), the marker's height, the name's width at the
  marker's scale (never narrower than the 40-px cell), same tint, depth bias and blend. Plates: the label pool now
  shows the name's image in the label's tint; the words sit where the label's ink was (left/right edge by screen
  half); a name wider than the label's cell is condensed down to 70 % width, then shrunk uniformly round the words'
  middle (a 6-letter name of wide letters like WWWWWW ends up ~half height on the stats page; normal names fit at
  full height, see the shots).
- Select card (`roster.rs` `band`): a named seat's big "1P" is replaced by the name in the plate style (white,
  dark outline), as tall as the 1P's caps; the "NAME - device" line is just the device again. `Tex::Name(p)` +
  `names::NameArt` (rebuilt when a seat's name changes). `HST_MENU=names-card` shows it.
- `names::MAX` 10 → 6 (typing, the grid and names read back from `names.txt` / sent online are cut at 6).
- `menu_art::HEAVY` and `stamp` became `pub(crate)` (no other change there).
- Tests: `name_tags::tags_are_the_originals_size` ("1P" as a head tag = 40×32, as a plate ~the disc's width and
  19 tall; accents/descenders get their own rows; `HST_TAG_ART=DIR` saves them), `plates_fit_their_cells`; names
  tests updated for 6.

Looked at (`context/b541/`): `art/compare.png` (disc 1P cells vs ours), `card.png`, `match.png` (ANNA / KIM-LI next
to the disc COM markers — same look), `final3.png`, `stats3.png` / `stats4.png`.

## Not verified / not 1:1

- Font: not the original's (it has no full alphabet); M PLUS Rounded ExtraBold set in the tags' colours/outline/
  slant/size, so letter shapes are rounder than the disc's 1P. Unavoidable without the original's glyphs.
- Not compared against PCSX2 at the same moment: the comparison is against the disc's own tag art, which is what
  the original draws (presentation/HUD: by eye per PLAN).
- Head tag pointer is drawn as a vector triangle matching the disc's pixel measurements, not cut from the sheet.
- Long, wide names on the plates (stats page: 48-px cell) get condensed then shrunk — smaller than the 1P tag.
- The select card tag is white (not tinted) on the seat-coloured band: the menu is ours; a seat-tinted fill on the
  seat's own colour would vanish.
- The in-match score panel's own "1P" beside the face still says 1P (not in this task, as in B371).
- The point boards (between points) share the final board's code path (`Tex::Slot`) and weren't shot separately.
- Lowercase descenders on the head tag run into the pointer's top (the disc's tags have none to copy).
