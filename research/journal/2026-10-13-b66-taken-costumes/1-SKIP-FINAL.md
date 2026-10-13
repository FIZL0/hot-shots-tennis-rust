# B66 — character select skips taken costumes

`crates/hst/src/play/main_menu.rs`:
- `Menu::free` / `next_free` (wrapping, either way) / `skip_taken`: a costume is free to a player when it loaded and
  nobody else is ready on it (disc costumes, costume packs and mods alike; `blocked` is now `!free`).
- After every select step and right after a player readies, every player not ready on a taken costume moves on to that
  character's next free one. L1 / R1 jump past taken (and M1n's failed-to-load) costumes.
- Left/right/up/down pass over characters with no free costume: on the disc grid `grid_step` is repeated the same
  way; on the custom page the ±1 / ±row step is repeated. If nothing free lies that way the cursor stays (the grid and
  the list don't wrap at their edges today, so the character skip doesn't either).
- Such characters are drawn dark (tint 50/128) on the grid and the custom page, judged for the player picking.

Tests: `locked_costumes_and_match_order` (1P readies Ashley costume 1 → 2P moves to costume 2; away and back lands on 2;
L1/R1 pass 1; one-costume mods with two taken are skipped by ←→↑↓ and the list start holds), `custom_page_picks_a_mod`
updated for the auto-move.

## Not verified / not 1:1
- The original's rule wasn't lock-stepped: B40's port of it shows "Not selectable" on a taken costume and blocks
  readying (that's what this replaces, by the user's request). PLAN: the front-end needn't match the original.
- "Otherwise unavailable" characters: there's no notion yet of a mod not allowed in an online lobby on the select
  (online.rs has no such filter), so only "no free costume" (taken or failed to load) counts. A future filter only
  has to make `free` return false.
- Cursor skip stops at the grid/list edge instead of wrapping (matches the existing non-wrapping cursor).
- The dimming wasn't screenshotted: no `HST_MENU` shot puts a character fully taken (disc characters have 10
  costumes for at most 4 players; only small mods can fill up).
