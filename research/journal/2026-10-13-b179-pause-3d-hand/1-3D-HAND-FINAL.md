# B179: the pause menu points with the 3D hand

The pause dialog (and its "Leave the match?" confirm, which draws the same `HAND` quad) and the Settings screens
over the match now use the main menus' 3D hand (`main_menu/cursor.rs`, `MENU00B.XB0` `menu/3d/cursor` playing
`cursor.ani`) instead of the flat `hsm_yubi` sprite.

- `play/menu.rs` `plugin`: runs `cursor::setup` (Startup) and `cursor::tick` (after `draw`) in the match mode too.
  Its render layer 8 is free in a match (the umpire call uses 7).
- `play/menu.rs` `draw`: with the cursor up, the `HAND` quad shows its image, padded `PAD`× about its centre as
  the main menu does (B146), at the main menus' row offset (row x − 60, y + 3; `hsm_yubi` sat at x − 48, y + 8),
  and the 16 px `hsm_yubi` swing is off (`hand_x` 0): the 3D hand wags in place. `layout()` is unchanged, so with
  no cursor (no disc model) it falls back to the sprite and its swing.
- Moving between rows restarts `cursor.ani` (still, then wagging) because `cursor::tick` already restarts on any
  change in where its image's visible nodes sit; the pause slots are such nodes.
- The pause Settings overlay (`main_menu/pause.rs`) draws through the main menu's `draw`, which already used the
  `Cursor` resource when present; it just never existed in a match. Now it does.
- The test-only pause plugin app (`open_change_back_in_a_match`) doesn't build `menu::plugin`, so it doesn't need
  the mesh/material assets the cursor's setup takes.

## Checked (`--shot`, `HST_PAUSE=6 --play --shot-at 9`, files in `context/b179/run1/`)
`pause2-43.png`, `pause2-169.png`, `pause2-219.png`: the 3D hand beside the chosen pill at 4:3, 16:9, 21:9;
`settings-169.png` (`HST_PAUSE_SETTINGS=settings`): the 3D hand on the Options rows.
The pills themselves are still the broken black bars of B177 (not this task).

## Not verified / not 1:1
- The quit confirm wasn't shot (needs a scored point first); it uses the same `HAND` quad and code path.
- The hand's restart-on-move wasn't watched live in a match, only reasoned from `cursor::tick` (same as the main menus).
- Position next to the pills is ours (the main menus' offset); the original's pause menu has the 2D sprite, so
  there's no original to match (menus are ours to design, PLAN.md).
