# B282 — mod names drawn like disc names in the online lobby

**Cause.** Before B295 the lobby's seat list drew a disc character's name from the disc's name sheet
(`characterselect2_02`, `NAMES` quads: the original's lettering) and a mod's name with our font (`d.fitted`/`d.centred`),
so the two looked different (font, outline, size).

**Fix.** Already in main via B295's lobby redesign (27ca7860): every seat row goes through `main_menu::seat_row`, the
name a string (`CHAR_NAMES[c]` for a disc pick, the mod's own name otherwise) drawn by `left` in our font at 17 px,
white (grey while picking). The character select's cards and the confirm screen's rows already used `name_of` → our
font for both. No code still draws the `NAMES` sheet in the menus.

This task adds `play::main_menu::tests::seat_names_one_style`: a seat row for `Ashley` and for `Anya` draws only font
glyphs, no sheet art, same height, colour and alpha.

## Not verified / not 1:1
- No side-by-side screenshot of a live online lobby with a mod pick: it needs two peers with a mod installed; the
  drawing path is shared code, so the unit test covers it.
- The front-end is ours (PLAN.md), so there is no original to match for mod names.
