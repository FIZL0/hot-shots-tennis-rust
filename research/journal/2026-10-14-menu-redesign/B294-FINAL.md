# B294 — Settings redesign (FINAL)

Mockup: `mockups/Settings.dc.html` (main checkout's journal dir).

## What changed
- The Settings hub screen is gone: main menu row "Settings" (title band renamed from "Options") and the pause
  overlay open straight on the Display tab.
- Tabs across the title band (Display, Graphics, Sound, Controls, Online & Mods) — L1/R1 (Q/E) switch, wrapping;
  tapping a tab opens it (`pointer::Act::Tab`).
- Each option is one compact row: label left, `< value >` box right (volumes also draw a slider bar); the selected
  row is the yellow cell with the hand. Tapping a value's left/right half steps it.
- Side panel: selected option's name, description, its choices as chips (current one dark) and, for options a
  Preset sets, the "Set by Preset…" note. Controls' panel lists the action's bindings.
- Description ticker removed; slim footer legend: ↑↓ Option, ◀▶/✕ Change, L1 R1 Page, ○ Back
  (Controls: Action, Rebind, Page, Back).
- Layout from the Grid extents: side panel right in landscape, below the list in portrait; rows shrink to fit.
  Pause overlay now fits its Grid to the window aspect too (`pause::fit`); its quad pool grew to 1200.

## Checks
- `tools/check.sh -p hst --bin hst` → 287 passed; `options::every_page` (tabs, wrap, values ⊂ choices, back),
  `options::pages_fit` (aspects 4 … 0.4: rows above footnote, panel ≥150 wide, quads within the pause pool).
- Shots (`context/b294/`): graphics 1280×720, 960×720, 540×960; sound 1280×720, 1680×720; controls 1280×720.
  Compared to the mockup by eye.

## Not verified / not 1:1
- Front-end is ours (PLAN 2026-10-12): compared to the mockup by eye only, nothing against the original.
- Mockup's side-panel preview images ([Off]/[High] placeholders) left out: no art for them.
- "Reset page" button left out: no reset-to-defaults per page exists yet.
- L1/R1 key caps beside the tabs left out; the legend shows the device's real keys (Q/E on keyboard).
- Tab labels shrink at 4:3 and portrait so all five fit.
- Choice chips are display only (rows/arrows change values).
- Controls bindings still shown by raw name (KeyW, PadDPadUp), as before.
