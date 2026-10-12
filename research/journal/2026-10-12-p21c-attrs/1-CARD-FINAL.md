# P21c — △ attribute card letters

## Cause
The grades were right; the glyphs weren't. `exe::Menu::grades()` reads the MENU overlay's 14×5 table, and the
original's card reads that same table with the highlighted character id (its grade getter is `table[char*5 + stat]`,
the sprite frame is `5 - grade`). Read live over PINE at the doubles character select (slot 2), the table equals
the disc's byte for byte. But the port drew each letter from column 0 of `CharacterSelect2_02.tm2`, which is the
character *names* sheet: it showed the left edge of a name ("As", "Co", …), not a letter.

## The original's card (MENU overlay sprite records, sheet `CharacterSelect2_04.tm2`)
Records are (sheet, w, h, u, v, dx, dy); frame n steps v by h.
- title box 144×40 at v 0 (fill, tinted per player) + v 40 (edge), drawn at (2, 2);
- play-style title 144×40 at v 80 + 40·style (All-round / Baseline / Net / Big Server Player);
- labels Serve…Footwork 112×120 at (0, 248), drawn at (16, 40);
- letters 24×24 at u 232, v 24·(5 − grade) (A at the top … F), drawn at (104, 40 + 24·row).
- title box tints (runtime, read over PINE): 1P (100,39,39), 2P (82,63,27), 3P (23,69,79), 4P (67,72,25).

## Port
`main_menu.rs attribute_card` draws that over the panel in place of the portrait, style and name; the 1P/costume
tags are hidden while it's up. `HST_MENU=attrs` opens the select with 1P's card on (for `--shot`). Mod characters
keep their TParam number card. Test: `hst-data/tests/menu.rs` checks all 14 characters' letters.

## Not verified / not 1:1
- No side-by-side screenshot of the original's card this run: the desktop was on a Hyprland recovery instance with
  no client windows, so PCSX2's F8 shortcut couldn't be sent (`tools/screenshot.sh` "wrote no screenshot").
  Letters are checked against the live table (PINE) and the P21b on-screen reading (Ashley EFEAD); layout comes
  from the original's sprite records, positioned by eye inside our panel (menus need not be 1:1).
- The title box tint per player was read from RAM at slot 2 (P1 only seen in use); which field selects it
  (player vs. seat) wasn't traced — the port uses the panel's player.
