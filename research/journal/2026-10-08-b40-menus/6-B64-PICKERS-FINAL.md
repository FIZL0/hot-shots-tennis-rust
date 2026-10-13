# B64: court and umpire pickers like the original (2026-10-13)

Original walked on PCSX2 copy 2 (save slot 2 = doubles character select; four ✕ → confirm screen; saved to scratch 9).
Shots: `context/b64/run1/` (singles walk, c2–c5 court picker, u1–u2 umpire picker), `run2/p00–p16` (all 11 courts and
the umpire cards, cropped from the copy's `snaps/`: `screenshot.sh`'s copy came out truncated on some shots).
Port shots: `context/b64/port_{courts,umpires,confirm}.png` (`HST_MENU=courts|umpires|confirm`, via an HST_BIN wrapper).

## What the original does
- ✕ on **Select Court** swaps the 3×2 row grid for a picker: the gold "Select Court" title (`Confirmation_03`), a strip
  of five court tiles (`Confirmation_02`, 92×68 at (96·col, 72·row)), the hovered one in the middle larger and lit, the
  others dark, each with its number (the sheet's digits, 20×20 from (192, 56)), ◀ ▶ at the ends (hidden while it slides).
  ←→ slide the strip and wrap (11 → 1); the court name, surface, meters and backdrop follow the hover at once. ✕ keeps,
  ○ puts the old court back. Ticker: message 349 "Choose a court for this match." (the closed row: 326).
- Strip order (tile n → message court k, 276 + 2k): 1 North Academy 0 · 2 Sakuragaoka 3 · 3 Alpine 5 · 4 Aloha 1 ·
  5 Wild Green 7 · 6 Nakagawa 8 · 7 Miyabi 10 · 8 Olympia 9 · 9 Western 2 · 10 Grand Palais 6 · 11 Queen's 4. The sheet
  holds courts 0–6 down its left column and 7–10 down its right (tile 11 = "?").
- ✕ on **Select Umpire**: "Select Umpire" title, five face cards (`Confirmation_03`, 80×64 at (8 + 88u, 392), names
  baked in), the hovered one larger and raised; ←→ stop at the ends (no wrap); the header face/name follow the hover;
  "△ Voice Sample" pill at the right. Ticker 333 (closed row: 328).

## Port (`play/main_menu.rs`)
- `Menu::picking` (the value the picker opened on); a picker arm in `step` before the confirm arm; `STRIP` order;
  `picker()` draws strip / cards in place of the rows. ←→ on the closed Court/Umpire rows no longer change them.
- Our Random stays as one more entry: after tile 11 on the strip, after Anna on the cards (the sheet's "?" tile).
- Tests: `court_and_umpire_pickers` (strip order, wrap, ○ restores, umpire clamp), `random_court_and_umpire` updated.

## Not verified / not 1:1
- No court backdrop: the original draws the hovered court's whole 3D scene behind the confirm screen (and the players
  stand in it); the port keeps its green grid and 3D previews. Rendering a court in the menu process is a large job and
  menus needn't be 1:1 (user 2026-10-12). There are no separate "court preview models" in the original to port.
- No △ Voice Sample (the menu loads no umpire voice banks); the pill is left out rather than drawn dead.
- No strip slide animation and no arrow hiding while it slides; positions/sizes measured by eye from 16:9 PCSX2
  shots (scaled to 640×448), not from the overlay's draw code.
- Sounds: move 0 / confirm 2 / back 1 as the rest of the menu, not checked against the original's picker by ear.
- The original's court descriptions (581 + 2k) aren't on this screen in the original; the port no longer shows them.
