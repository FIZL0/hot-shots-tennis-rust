# P21b — character select costumes and attributes (BLOCKED on P21)

## Why blocked
P21b adds L1/R1 costumes and △ stats *to the character select menu*, and the port has no menus yet: P21
(menus and modes, incl. character select) is open. `main.rs` takes characters and costumes from `--chars` /
`--outfits` only, and `play.rs character_hand` notes the switch-hand toggle waits for the menu too. Building the
whole character select screen here would be P21's work; do P21's character select first, then this.

## What the original does (slot 2, doubles character select, P1 on Ashley; screenshots in `context/p21b/`)
- P1's panel (top-left, pink) shows a **2D portrait** of the highlighted character in the current costume, a
  shirt icon with the costume number (Ashley starts on 3), the hand tag ("Left handed", from the switch-hand toggle),
  type ("All-round") and name. The bottom ticker scrolls the character's description.
- **R1** stepped the costume 3 → 4 and the portrait changed live (green outfit → yellow). The header's hint reads
  "L1 Select Costume" in the normal view and "R1 Select Costume" in the attributes view (needs a closer look:
  which shoulder steps which way, wrap-around, locked costumes). Pressed R1, R1, L1 and it ended on 4.
- **△** toggles the panel to an attributes card ("△ Show Attributes" in the header): title = the type
  ("All-round Player"), then five rows with letter grades: Serve, Stroke, Volley, Impact, Footwork (Ashley:
  E, F, E, A, D). △ again goes back to the portrait. The grid cursor stays put.
- Each player has their own panel (1P top-left, 2P bottom-left, 3P/4P right), so both work per player.

## Leads for the port
- Grades: not a TParam.csv column (TParam has Serv/Strk/Voley POW/CON, SPE, Agili, STA … and a total `計`);
  find the menu code that fills the card (a table or a banding of TParam values).
- Art: `MENU/MENU12A.XB0` has `Wall_/Title_/KeyAssign_CharacterSelect2.tm2`; `CharacterSelect2_01.tm2` is in
  MENU04/50/60; the per-costume portraits are still to be located.
- Costumes in the port: `--outfits` (0..9) already selects model, panel face and AI row, so the live swap needs
  the character's model reloaded per costume; M1 mod costumes extend the list when M1 lands.
