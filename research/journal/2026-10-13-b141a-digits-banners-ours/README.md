# B141a — our own score digits, banners, VS and icons (FINAL)

Follow-up to B141 (j: 2026-10-12-b141-menu-art-ours). The last disc art the menus still showed upscaled is now drawn
by us, in B141's style.

## What changed (`play/menu_art.rs`, `play/main_menu.rs`)
- `Sheet` gained `stamps` (cells re-drawn as stylised text) and `icons` (a sheet-wide painter). `image()` clears each
  stamp cell (+4 px), then `stamp()`s our text into it: our HEAVY font, cap height fits the cell, a white ring, a dark
  edge, a vertical colour fade, optional slant. Colours are a `Look` per stamp set, all consts in one place.
  - `result_gameset01` / `02` digits 0–9 and "-" (past green, current orange) — pause and result score boards.
  - `inpane_pause00` / `01` point words 0/15/30/40/D/A and tiebreak 0–7/D/A (gold and red, slanted).
  - `result_gameset05` banners: DOMINATION, ANNIHILATION, VICTORY, DEFEAT, NARROW WIN, NARROW LOSS.
  - `confirmation_03` "VS" over the flame.
- `info_icons` draws `info.tm2`'s right half ours: L1/L2/R1/R2, d-pads (each arm lit + plain), △ □ ○ ✕, the
  orange dot, START/SELECT plain and lit, four arrows. `image()` now normalises the name (`/2d/info.tm2` → `info`).
- `pill_layers` grew an icon strip (`BALL`, `BALL_OFF`, `BOUNCE`, `SPEED`); the confirm screen's court meters
  draw from it instead of `confirmation_01`.
- Fix: the main menu's quad pool (`POOL`) was 700 and overflowed on the confirm screen (everything after the 3P
  pill — VS, rows, umpire — was dropped). Now 1400.
- Test `stamps_and_icons_ink_their_cells`; with `HST_MENU_ART=DIR` it also writes the meter icons to `DIR`.

## Checked
Sheet dumps (`HST_MENU_ART`) and `tools/shot.sh` shots in `context/b141a/run2`, `run3`: pause board (4:3 and
1600×600), result banner, confirm screen with VS. Shots need `--shot-at` (confirm ~3 s, pause ~7 s, stats ~24 s).

## Pitfalls
- Extracting the whole disc with xbdump into the scratchpad hits the disk quota; dump only the sheets you need
  (`HST_MENU_ART`).

## Not verified / not 1:1
- Colours, shapes and banner/icon designs are ours; no PCSX2 side-by-side (menus/HUD art aren't 1:1 by rule).
- "OK!": not found on any sheet `menu_art` rebuilds; left as is.
- `popups.rs`'s digits (`result_gameset00`/`03`, and `result_gameset02`'s white backing row) go through
  `panel::image`, not `menu_art`; they stay upscaled disc art.
- `info.tm2`'s left half (plain boxes, Yes/No) stays upscaled disc art.
- The meter icons and balls live on `pill_layers`, so `--original` shows ours too.
- Meter icons were checked in the test's sheet dump only (`HST_MENU_ART`, pill_icons.png): `HST_MENU=confirm` opens on "Random Court", which draws no meters.
- The info icons in place (chars-screen hints, result key bar) were checked on the sheet dump, not in a shot.
