# B498: Settings dropdowns, Preset pinned on top

Concept: the user's approved artifact (copy in `context/concept-settings-dropdown/settings-dropdown.html`, main checkout).

## What changed
- `options.rs`: rows with ≥5 choices (`listed`: Preset, Resolution, Frame Limit on Graphics; Aspect Ratio, Menu Size,
  Button Icons on Display; Downloads at Once on Online & Mods) show "value ▼" and open a dropdown on ✕: ↑↓ move
  (wrapping), ✕ picks (applied + saved, only if it changed), ○ closes unchanged; ←→ on the unopened row step it at
  once (saved). Sliders, Advanced and Renderer never list. ≤4-choice rows keep B326's pick-then-‹ › cycle.
- The list sits under the row (over it when there's more room there), right-aligned to the value pill, inside the
  list panel (portrait: down over the side panel too); choices closer together where they'd run past. Current
  choice has a blue dot, the highlighted one is yellow. Preset's entries carry hints (High: full quality, no
  speed-ups · Default: High with speed-ups on · Steam Deck: tuned for the Deck · Low: less detail, more fps ·
  Lowest: half res, no shadows · Phone: for phones and tablets · Auto: picks and tunes itself · Custom: your own
  mix), Auto/Custom under a rule. Picking Custom changes nothing.
- Graphics: Preset alone above the rest, "START HERE" label over it, dashed divider under it (`page_row`, `pin_marks`).
- Mouse/touch: the open list's choices are `Act::Row(j, OK)` (hover highlights via `Menu::choose`, tap picks); a tap
  off it closes (popup outside = ○). `Menu::listing` holds the highlight; `editing` is set too so ↑↓ don't move rows.
- Legend: unopened dropdown row "Option / Change / Open list / Page / Back"; open "Move / Select / Close".
- `HST_MENU=graphics-list` / `graphics-fps` open Graphics with Preset's / Frame Limit's list (for `--shot`).

## Checks
- Tests: `options::tests::{every_page (list_row), dropdowns, pages_fit (lists open, quad pool)}`, legend's
  every-state probe (lists open on each listed row), pointer and pause tests updated for the new row places/flow.
- Shots (16:9 and 540×960 portrait) in `context/b498/`: lists drawn inside the panels, hints readable.

## Not verified / not 1:1
- Not compared with the original: the Settings screen is ours (front-end, PLAN.md); this follows the user's concept.
- The concept's slightly larger Preset row font isn't copied (the row keeps the list's size).
- The pause screen's embedded pages use the same code but weren't screenshotted in a match.
