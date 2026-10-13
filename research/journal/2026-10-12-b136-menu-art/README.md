# B136 — remastered menu art

## What landed

`crates/hst/src/play/menu_art.rs` rebuilds the menu sheets at load time at 4× their disc size, keeping the same
layout. Each screen's pixel rects still pick the same pieces because `textures::ours` registers the 4× scale and
`ui_rects` multiplies every `ImageNode` rect on those images.

For every label in a sheet's table:
- the disc label's ink box is erased row by row (each row interpolated between the pixels just left and right of
  the box, so stripes and pills behind it survive);
- the words are set again as vector text (tiny-skia + ab_glyph) in M PLUS Rounded 1c ExtraBold (Bold for `word.tm2`'s
  pill font);
- the fill gradient (per row), the outline colour and the outline width are sampled from the disc label itself.

Labels under 10 px tall get no outline. Title bars get a drop shadow and a wider erase to cover the old one. The rest
of the sheet (pills, boxes, icons, portraits) is upscaled bilinearly with snapped edges.

Only the label boxes and their words are in git. The pixels come from the user's disc at run time, so nothing traced
is committed. The fonts are OFL (`crates/hst/assets/fonts/OFL.txt`), subset to Latin (about 100 KB for both). Every label string is a fixed table, so the subset covers them all; a new label with non-Latin characters needs a fuller subset.

Sheets covered:
- **Title bars**: Main Menu, Options, Game Selection, Character Selection, Confirm Settings. The box is found at run
  time from the bar's stripes.
- **Menu font**: `word.tm2` (Local/Online/Settings/…/Back pills, ON/OFF). The glyph boxes come from `main_menu::FONT`.
- **Mode**: option_00 (Singles/Doubles).
- **Character select**: characterselect2_00/02/04 (card labels, handedness tags, names, the attribute card) and the
  keyassign hints.
- **Confirm screen**: confirmation_01/03/04 (courts, confirm labels, ranks).
- **Dialog**: i_dialog_01.
- **Loading**: "Now Loading...".
- **Pause**: inpane_pause02 (Paused, Resume/Settings/Return pills).
- **In-match**: inpane_team (A/B Team).
- **Result**: result_status01 (stat names) and keyassign_inpane (Continue / Score Screen / Stats Screen).

Options:
- `--original` keeps the disc sheets.
- `HST_MENU_ART=DIR` dumps every rebuilt sheet as PNG.

Tools:
- `research/b136_labels.py SHEET.png` gives a first OCR guess of a sheet's label boxes.
- `research/b136_tight.py SHEET.png 'x y w h Text'…` tightens rough boxes to the ink and prints the table lines.

## Checked

4K shots (3840×2160, `tools/shot.sh` with `HST_SIZE='2400 1350'`) are in `context/b136/`:
- `k_main`, `k_settings`, `k_mode`, `k_chars`, `k_attrs`, `k_confirm` (each with a `_half` downscale);
- `k_pause` (`HST_PAUSE=6 --shot-at 8`), `k_result` (score page, `HST_STATS=18 --shot-at 30`) and `k_result2`
  (`HST_STATS_PAGE=1`);
- `k_loading` (`HST_MENU=confirm` plus an `HST_KEYS` Enter).

Every remastered label is sharp at 4K (`k_result2_crop.png` is a 1:1 crop).

The side-by-side against PCSX2 screenshots (`tools/screenshot.sh` frames from the B135 run,
`context/b136/side_by_side.png`) shows the same bars, pills, colours and outlines. The port's own screens (Local/Online,
Settings) are our design (PLAN: menus need not be 1:1); their pills use the remastered disc font.

`tools/check.sh -p hst`: the new `menu_art::tests::label_round_trip` passes. `net::tests::mod_looks_peer_to_peer_and_the_ready_gate`
timed out once under load (other slots were building) and passes when run alone; it is unrelated.

## Not verified / not 1:1

- **Fonts are an approximation.** The original uses a heavy, squarer sans for titles and pills, plus condensed faces
  for the ranks and key hints; we have no licence-free copy. M PLUS Rounded 1c is close in weight and roundness, and
  narrow labels are squeezed horizontally to fit the disc box. Our titles read a little lighter and smaller than
  PCSX2's "Character Selection" / "Game Selection".
- **Colours are sampled, not specified.** The per-row gradient and outline come from the disc pixels, so labels that
  are anti-aliased or tiny (the 7–9 px handedness tags) can come out a shade off.
- **These are only upscaled (4× bilinear, edge-snapped), not redrawn**, because they are stylised artwork rather than
  text in a font we can set:
  - the stylised digit sheets (result_gameset00/01/02 score digits, the VS and OK! buttons);
  - the result_gameset05 banners (NARROW WIN …);
  - the portrait name tags and digit rows on confirmation_01/03;
  - the L1/R1/△/✕ and S/W icons;
  - the BEG/INT/EXP side tags and the small 1P–4P/COM tags on characterselect2_00;
  - the HUD's COM/Lv tags and stat numbers;
  - walls, portraits and faces;
  - the loading hints, which are baked into each 512×256 hint picture.
- **The bottom ticker** still uses the port's monospace font (not part of the disc sheets).
- **Outline residue (fixed after review):** the disc outline's faint outer pixel lies 1 px past the measured ink box.
  Left behind, it showed as dashes above "S" and below "g" on Singles/Doubles and as a dark halo on the pause and
  dialog buttons, so the new outline looked cut off. Labels with an outline are now erased with a 1 px margin
  (`rebuild`); see `context/b136/k_mode2.png` and `k_pause2.png`. Tiny unoutlined tags keep the tight box.
- **Fixed after review:**
  - **Pause "Settings":** `pause::label` builds this row at 1× from word.tm2 glyphs. It is now set again as vector text
    (`menu_art::relabel`, same fill and outline sampling as the dialog rows); `k_pause3.png`/`k_pause4.png`.
  - **Singles/Doubles pill rims:** the mode screen and the character select bar drew the pills from
    [2, 2, 252, 30], cutting 2 px off every side, so the dark rim was missing top and bottom. They now take the whole
    pill, [0, 0/32, 256, 32], scaled to the same place and size (`main_menu.rs`); `k_mode3.png`, `k_chars3.png`.
  - **Singles/Doubles pills redrawn as shapes:** upscaled, their round ends stayed jaggy. `menu_art::PILLS` now
    draws each as three anti-aliased stadiums (2 px dark rim, 2 px white ring, flat fill), colours sampled from the
    disc bar, before the label is set. The stray marks under the Doubles pill came from bilinear sampling into the
    1P tags at disc row 64; its source rect now stops at 31.75 px. `k_mode4_crop.png`, `k_chars4_crop.png`.
  - **Pause "sets"/"games":** the "games" box in `PAUSED` was 82–106, straddling the "sets" cell (64–88) and its own
    (88–112), so the new word was cut in half and spilled into "sets". Boxes re-measured from the sheet's ink:
    sets 67–86, games 92–111. `k_pause5_crop.png`.
  - **Our own button pill:** `Draw::pill` (main menu, Settings, Graphics, Controls, online rows) used Option_00's
    plain white blob, tinted. It now draws `menu_art::pill_layers`, a sheet we make ourselves (three white
    stadiums: whole, inset 2 px, inset 4 px), as three tinted layers, so rim, ring and fill each take their own colour
    (`Draw::pill3`; `pill` = dark rim, white ring, the old tint as fill). 9 quads a pill, so the pause overlay's quad
    pool went 240 → 480. `k_pill_main_half.png`, `k_pill_settings_half.png`, `k_pill_graphics_half.png`. The rest of
    the menu art is B141.
- **Pre-existing overlap:** the "Tab / Square: 92 custom characters…" hint overlaps the character grid. That is not B136.
- **Umpire select:** the port has no separate umpire select. The umpire is a row on the confirm screen, which is
  covered above.
