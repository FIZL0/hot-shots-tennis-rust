# B462 — L2 / R2 section switching stops at the ends

User 2026-10-15: L2 / R2 (keyboard 1 / 3) never wrap.

- Character select (`play/main_menu/tabs.rs`): the jump's target is now `jump_to` (pure: L2 the section's own
  first open tile while past it, else the nearest earlier section with an open tile; R2 the nearest later one; None
  at the ends). `grid_move` uses it; `section_ends(p)` tells the legend which half does nothing.
- Mods page kind tabs (`mods_kind.rs`): `step` stops at the first / last shown tab; `ends` says which press is dead.
  The title band's L2 / R2 art draws that cap at `legend::DIM_ALPHA` (45 of 128).
- Legend (`legend.rs`): `Item::dim` fades a pair's first / second glyph (`fade`, premultiplied alpha × 45/128); set
  for `Act::Kind` on the select (per player) and the Mods page. The select's one shared bar dims a half only when
  it does nothing for every seated player. `key_hints.rs` only gained the new field.
- previews.rs and mods_screen/grid.rs had no L2 / R2 wrapping left (B422 moved the preview voices to the right
  stick; grid.rs only documents the tab row): nothing to change there.
- Tests: `tabs::l2_r2_jump_sections` (both ends stay, the legend dims R2 on the last section, Favorites first),
  `mods_kind::tabs_stop_at_the_ends_and_say_what_is_coming`, `mods_screen`'s kinds test (R2 on Umpires stays).
- Look: `--shot` of the select with 1P jumped to the last section (temporary, uncommitted `HST_MENU` hook): the
  legend's R2 cap is faded, L2 full (context/B462/last_zoom.png).

## Not verified / not 1:1
- The Mods page's dimmed tab-row cap wasn't screenshotted: no `HST_MENU` opens the Mods page; covered by `ends`'
  unit test and the same alpha path the row already used.
- Not 1:1 by design: menus are ours (PLAN.md); the original has no such section switch.
