# P23b — anti-aliasing (MSAA) setting

Settings → Graphics gets an *Anti-aliasing* row: Off / 2x / 4x (settings.txt `anti_aliasing = off|2x|4x`, flag
`--msaa off|2x|4x`). Default 4x = Bevy's default, so the default look is unchanged. Presets: Default and Steam Deck
4x, Low off. `graphics.rs` `msaa` sets every `hud_gamma::Scene` camera (the court's and `popups.rs`'s call overlay)
to the same count.

Why it blanked before: not the shot timing, and not mismatched counts alone. `hud_gamma::clear` gave *every* Scene
camera the encoded clear colour, including the call overlay (order 1, meant to be `ClearColorConfig::None`). With
MSAA off both cameras share the main texture directly, so the overlay's clear wiped the court (a `--play --msaa off`
shot showed only the HUD; with the overlay camera disabled the players came back). `clear` now leaves `None` cameras
alone. Shots in `context/p23b/run4` (`--play`, off/2x/4x, all draw the players) and `run5` (`--stage 1 --play`, off vs
4x: net tape jagged vs smooth).

The Graphics screen now has 8 rows: drawn 42 px apart from y 66 (`rows_at`) so Back clears the description bar.

## Not verified / not 1:1
- Remaster-only setting: no original to compare against.
- The Graphics screen's new layout (8 rows, 42 px pitch) wasn't screenshotted: driving the menu to it needs input;
  checked by arithmetic only (last pill ends y 398 < bar at 404).
- 2x on WebGL2 / unusual adapters isn't checked: wgpu panics if an adapter doesn't support 2 samples for the HDR
  format (desktop Vulkan here is fine).
- The call-model overlay itself (Out/Fault models) wasn't shot mid-call with MSAA off; it now keeps `None` and draws
  over the court as at 4x.
