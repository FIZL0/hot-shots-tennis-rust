# B141 — our own menu art for the rest (FINAL)

The menu pieces that were still disc art (upscaled or rebuilt from it) are now drawn from one vector sheet of ours,
`menu_art::pill_layers`. It holds the B136 pill's three layers plus three rounded-box layers (rim, ring and fill,
each white and tinted separately). They are placed with `pill_quads` (three-sliced, ends h/2 wide, so any height
works) and `box_quads` (nine-sliced, any corner size).

| Piece | Now |
|---|---|
| Title bars (all five screens, online too) | a band in the screen's colour, white and dark rules, a box tab hanging from it, the title in our font with a soft shadow (`TITLES`) |
| Mode Singles/Doubles bars, the chars screen's top bar | `mode_bar`: a pill in `MODE_FILL`, the word in our font; the unchosen one dimmed and see-through |
| Character cards and the assign-screen cards | `box3` in `CARD_FILL[p]`, darkened when nobody is picking |
| Hand tag | a small pill "Right hander"/"Left hander", gold when swapped |
| Play style, name, "Ready to go!", "Not selectable" | gold/grey text (`STYLE_NAMES`, `CHAR_NAMES`) |
| Rank pills (select + confirm) | a pill in the player's colour + `RANK_NAMES` |
| BEG/INT/EXP groups, custom list frame | box outlines plus a round bar with stacked letters |
| Key hint words, Settings ON/OFF, "Umpire", court surface, "Start the match!" | text |
| Pause dialog (`play/menu.rs`) | a box (dark brown fill) and our pills, chosen [123,105,79] / not [94,83,70] as before; all three rows set in our font via `pause::label`; POOL 160→200 |

Also: the pause-over-match Settings screens (`main_menu/pause.rs`) never pushed the pill sheet, so `Tex::Pill`
(`art.last()`) resolved to `option_00`. It now pushes `pill_layers` last, as the main menu does.

Shots: `context/b141/before/` (disc art) vs `context/b141/after3/` (main, settings, mode, chars, attrs, coms,
confirm, pause). They were checked by eye: no overlaps, everything legible, and the layout holds in a wide window
(the title band spans it, the tab stays left).

Pitfalls:
- `--shot` exits as soon as its output file exists, so re-shooting into the same directory silently keeps the
  old PNGs. Shoot into a fresh dir.
- Another slot's build overwrote the shared `target/release/hst` between my build and the shots, so one round
  showed the old art. I built into the scratchpad's own target for the final shots.

## Not verified / not 1:1
- The colours, shapes and layout are ours, not sampled from the disc. Menus aren't 1:1 (PLAN, user 2026-10-12),
  so there is no PCSX2 side-by-side.
- `--original` no longer brings back the disc's title bars, mode bars, cards, tags, rank pills, group boxes or
  pause dialog. These pieces are now drawn as ours either way; `--original` still keeps the disc sheets that are
  still used (font, buttons, icons, VS).
- The stylised digits, result banners, "VS", "OK!", button and meter icons are still the disc's upscaled art.
  They are queued as B141a; `result.rs` was not in this task's files.
- The attribute card (△) title box, stat names and grade letters are still B136's rebuilt disc sheet (`ATTRS`).
  It already reads as vector text, so I left it.
- Online screens were not shot. They call the same `Draw::frame` and helpers.
