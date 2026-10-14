# B212 — Sets and Games picked like Court/Umpire

## The original (PCSX2 copy 4, slot 2 → select → Start → Confirm Settings, scratch slot 8)

`context/b212/strip.png` (crops of `sets_*.png`): on the hovered Sets pill ←→ move the hand to the next pill in the
grid (Games), they don't change it. ✕ (○ on the US pad) picks the pill: the hand sits at its left and ◀ ▶ arrows
appear. Sets steps 1 → 3 → 5 and **stops** at the ends: at 1 the ◀ is gone and ← does nothing, at 5 the ▶ is gone.
Games does the same over 2/4/6 (B188's shots).

## The port (`play/main_menu.rs`)

- ✕ on any row but Start opens its picker; `picking` holds the old value (court, sets index, games, umpire).
  On the unpicked rows ←→ do nothing.
- Picked Sets/Games: ←→ step and stop at the ends (sound only on a change), ✕ keeps, ○ puts back. The pill turns
  blue with `<` `>` at its edges, each hidden at its end like the original's arrows. The other rows stay drawn.
- Mouse/touch: the first tap only picks the pill. Then its left/right thirds step it, its middle keeps it, and a tap
  off it puts the old value back (the court picker's popup rule).
- Legend (M14): Confirm rows 1–4 show "✕ Choose"; picked shows "↔ Sets/Games, ✕ Keep, ○ Cancel".
- `HST_MENU=sets|games` opens the confirm screen with that pill picked (for `--shot`).
- Test `sets_and_games_pickers` covers keys and taps. Shots: `context/b212/port_{sets,games}_{43,169}.png`.

## Not verified / not 1:1

- **The picked pill's look is ours** (blue fill, `<` `>` in our font), not the original's art. Menus don't have to
  be 1:1 (PLAN.md).
- **←→ on an unpicked row do nothing.** In the original they move across its 3-column grid. Ours is one column,
  and the task asks for this.
- **Games' stop-at-the-ends comes from B188's shots.** I didn't shoot it again this time; Sets was checked here.
