# B359: the menu hand's camera off outside the menus

## What landed
- `play/main_menu/cursor.rs`: the hand's camera (`CursorCamera`, order −1, its own image) only draws while a
  `Tex::Cursor` quad is visible. `tick` (Update) turns it off when no hand quad was visible; `show` (the mode's
  `Last`, after visibility propagation, before extract) turns it on the frame a hand shows, so a menu opening
  never shows a stale image. `show` only ever turns it on, so the loading screen's pipeline warm-up
  (`mode.rs` `warm`) still activates it and compiles the hand's pipelines for the pause menu.
  Registered in both the main menu's and the match's (`play/menu.rs`) plugins.
- `HST_FPS` draws line now counts only active cameras' meshes and says "N of M cameras active" (it counted
  every Camera3d before).

## Measured
Native, `tools/bench.sh … 30 --stage 3 --court 3 --shadows low`, 9070 XT, 1280×800, 4 bots, alternating
before/after binaries (machine load average ~18 from other agents, so fps swings 2×; cpu ms/frame steadier).
Means of the 10–25 s samples:

| run | before fps / cpu ms | after fps / cpu ms |
|---|---|---|
| 1 | 73 / 13.7 | 107 / 10.9 |
| 2 | 112 / 10.5 | 157 / 9.0 |
| 3 | 218 / 9.9 | 191 / 9.3 |

Draws: during a match 1 of 3 Camera3d active (the umpire-call camera was already off), meshes seen 286 → ~276
(the hand's meshes no longer drawn). CPU per frame 0.6–2.8 ms lower in every pair.

By eye (`tools/shot.sh`, context/b359/): the hand still shows on the main menu, the pause menu
(`HST_PAUSE=8`) and the pause menu's Settings (`HST_PAUSE_SETTINGS=settings`).

## Not verified / not 1:1
- Browser/mobile not re-measured: the machine is loaded by other agents' runs; the change only removes a
  render pass, so it can't cost anything there.
- The hand's directional light (render layer 8) still exists during a match; it lights nothing outside its
  layer and casts no shadow, left as is.
