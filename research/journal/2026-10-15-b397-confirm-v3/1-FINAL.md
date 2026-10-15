# 1 — Confirm v3 built (FINAL)

## What changed
- New `crates/hst/src/play/main_menu/confirm.rs`: the v3 frame for the local confirm screen.
  - Top line: the title's tab, "Doubles · 1 set, 4 games" in gold beside it, "Local match" (online: "Online match") right.
  - TEAM 1 · VS · TEAM 2 (singles: one card each side of VS); the menus' flame (`flame::draw`, drawing only — its
    RNG steps in `flame::tick` as before) behind a navy VS disc.
  - Card: seat tag, status chip (Ready ✓ green for a person, COM · Lv N white for the computer, its rank name
    without "Tennis "), 3D preview (`Tex::Preview(p)`, so `previews.rs` places the live/baked model as before),
    name, device (Keyboard / Controller N / Computer / an online peer's name), costume + hand chips, play style,
    pink stats card; a computer's rim dashed. Online stand-in note kept for a remote mod this machine lacks.
  - Side panel (180 units = the board's 300 at 1280): court card (picture, COURT, full name, "OK: change" when
    chosen), Mode / Sets / Games / Handicap pills two by two, umpire card (face, UMPIRE, name, a line: "Picked at the
    start" / "Umpire n of 5" / a mod's "Calls as …"). Portrait: the side panel goes under the teams (court left,
    pills over the umpire right), as B396's lobby.
  - Bottom bar: ‹ Back (pointer Back), the help line (the game's own message for the row as the head, the controls
    under it; a long head wraps both onto two lines), big "Start the match!" with "Everyone ready".
- `main_menu.rs`: `confirm()` now only dispatches (court / umpire picker screens unchanged, else `confirm::draw`);
  removed the VS-sprite stage with the four models (`stage`), the side card's court band, the Select Court / Sets /
  Games / Select Umpire button stack, `Grid::split` and its constants. `confirm_rects` → `confirm::rects` (Start,
  court card, Sets pill, Games pill, umpire card), so `Menu::sel` 0..4 and all the input code, legend and pickers are
  untouched; ↓ from the umpire card wraps to Start, ↓ from Start to the court card — the visual order.
- Tests: `confirm::tests::{equal_cards_either_side_of_vs, lays_out_at_any_aspect}` (8 aspects × singles/doubles:
  every part in the view, none overlapping, cards portrait and ≥ 180 tall); `lobby_and_confirm_fit_the_window`
  now checks the new rows instead of `split`. pointer.rs tests (court picker taps, Start) pass on the new rects.

## Sharing with B396
B396 (lobby v3) was still in progress in another worktree with nothing committed, rewriting `online/lobby.rs`
wholesale. Editing lobby.rs here would have conflicted with all of it, so `confirm.rs` carries its own copy of the
frame geometry (`teams`, `side_parts`) and card parts (`dotted`, `chip`, `stats`, `preview`, `tick`, colours),
written to B396's numbers so the two screens match. **B405** moves them into one module once both are in.

## Checked
Shots (`HST_MENU=confirm|confirm-random|confirm-court|sets|courts`, `tools/shot.sh`) at 1280×720, 1024×768,
600×1000 and 1600×600 in `context/b397/run1/`: layout as the board; Sets picked shows the blue pill with ‹ ›;
the court select is unchanged. `tools/check.sh -p hst` passes.

## Not verified / not 1:1
- Not shared code with B396 yet (see above; B405).
- An umpire mod shows its donor's disc face: B400 (a mod's own head, `umpire_mods::face`) isn't in main yet; B406.
- Mode and Handicap pills are display only (no ‹ ›): Mode was picked two screens back and changing it here would
  re-seat the players; Handicap is "Off" until B40g1. The board draws ‹ › on all four pills.
- Sets / Games keep B212's OK-then-←→ (the board's ‹ › suggest ←→ directly); kept so the pickers' behaviour,
  legend and tests are unchanged ("pickers unchanged").
- The board's 3D preview fills the well; ours keeps the preview bake's 4:5, so on narrow 4:3 doubles cards the model
  is smaller than the well.
- No PCSX2 comparison: the confirm screen is ours to design (PLAN: front-end needn't match the original).
