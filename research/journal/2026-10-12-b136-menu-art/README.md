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
- **The pause "Settings" label** on the in-match pause pill is built at 1× from word.tm2 pixels by
  `pause::label`, so it is upscaled rather than vector.
- **The bottom ticker** still uses the port's monospace font (not part of the disc sheets).
- **A faint speck under the Doubles pill and under "Clay Court"** is GPU half-texel sampling of the neighbouring white
  box on the sheet. It is there with the disc sheets too; it is not from this change.
- **Pre-existing overlap:** the "Tab / Square: 92 custom characters…" hint overlaps the character grid. That is not B136.
- **Umpire select:** the port has no separate umpire select. The umpire is a row on the confirm screen, which is
  covered above.
