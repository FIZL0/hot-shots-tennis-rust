# B40 main menu, settings, select (2026-10-08)

Code: `crates/hst/src/play/main_menu.rs` (new), `play/controls.rs` (rebinding, `--pads` seats), `play/widescreen.rs`
(`WIDE` switch, black bars), `main.rs` (menu when no mode flag; `--umpire --sets --games --pads --4x3 --no-upscale`),
`play.rs` / `play/npcs.rs` (umpire, sets and games from the flags instead of fixed), `hst-data/src/exe.rs`
(`Menu`: MENU.BIN overlay, grade table). Screens: `context/b40/{main,settings,controls,mode,assign,chars,confirm}.png`;
the original's for comparison `context/b40/00_charsel.png`, `05_setup.png`, `2x_attr_*.png`.

## Disc data used
- Art (all `..\data\menu\2d\X.tm2`): MENU10A Title_MainMenu; MENU70 Wall/Title_Option, Option_00 (ON/OFF, pills);
  MENU05 Wall/Title_PlayerSelect; MENU12A Wall/Title_CharacterSelect2; MENU12B CharacterSelect2_00..03 (mode pills,
  Ready/Not selectable, play-style labels, stat names, grade letters, names, faces, player cards) and the CS2_Shots
  portraits; MENU03 Wall_Confirmation, Title_Confirmation_01, Confirmation_03 (Start, VS, umpire faces); MENU60
  item_umpire00..04; MENU04 word.tm2 (font, 32×32 cells from y 32, 12 a row, ink widths from alpha).
- Text: MENU00B `message0.dat` (u32 count, u32 offsets relative to `4+4*count`, `07 7f` + Latin-1 + NUL): 188 Local,
  190 Settings, 192 mode, 193 controllers, 276+2k courts, 308+c characters, 323–328 confirm help, 581+2k court
  descriptions, 669+2u umpires.
- Grades: MENU.BIN overlay table (`exe::Menu::grades`, test `hst-data/tests/menu.rs`). Play style: TParam.csv col 7.
- Costumes don't change stats (user): only a computer player's level, not shown here.

## Checked
- `cargo test -p hst`: settings round trip, input repeat, costume lock + match order + flags, keyboard 1P only, grid
  moves, message parse; `controls` rebind + seated pads.
- Screens captured (grim; Bevy's `--shot` skips the menu window, "Unknown window for screenshot") and compared by eye
  with the original's character select and confirm screens.
- A match started with exactly the flags the menu builds for a doubles line-up (two Ashleys in costumes 1 and 2,
  Cody, Jun; `--pads -,1 --4x3 --no-upscale`) runs with those characters, costumes and black 4:3 bars.

## Not verified / not 1:1
- Layout, sizes and sprite rects are measured by eye from the sheets and PCSX2 screenshots, not from the menu
  overlay's draw code (B40e).
- No animation: the original's slides, the hand's swing (a plain sine here), the select's flame (P3f1a), the
  ticker scrolling of long descriptions (shrunk to fit here), the menu BGM (B40e).
- The back sound is SYS_SE00 key 1 by assumption (move 0 and confirm 2 checked by ear only).
- Costume 9's portrait sheet (`CS2_Shots_Cos_a/b`) is assumed; costumes 1–8 use `CS2_Shots_<char>`.
- Main menu, Settings and Controls screens are remaster-only (styled on the original's option screens); their
  English descriptions are ours.
- Court k → `--stage k+1` assumes the confirm screen's court order is the disc's stage order.
- Pad numbering across processes: the menu passes its sorted-gamepad index; the match sorts the same way, but gilrs
  could enumerate differently in the child (not tested with two real pads).
- The original's other choices (Set Handicap, Offbeat Rules, random court/umpire), the BEG/INT/EXP frames, the key
  hints and the CPU level on the confirm screen are missing (B40e/B40g).
- M7 cameras have no setting yet (B40g); only two humans; keyboard is 1P only (B40h).
- Menu ↔ match is a child process, not an in-process state change (B40f).

## Follow-up (user 2026-10-08)
- 4-player support is TOP PRIORITY: B40h (four humans, any device on any slot).
- Every guessed or by-eye detail above gets taken from the original, not guessed: B40i lists each one (sprite rects
  and positions, font spacing, sound keys, costume 9 sheet, court→stage order, play-style/grade mapping, umpire
  order, sets mapping, hand motion, pad numbering). Each value goes in this journal with its source.
