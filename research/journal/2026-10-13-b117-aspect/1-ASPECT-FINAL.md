# B117 Aspect ratio setting

Settings row 0 is now **Aspect Ratio** (Auto, 4:3, 16:9, 16:10, 21:9), replacing Widescreen on/off.
`settings.txt` stores `aspect = 16:9`; an old file's `widescreen = on` loads as Auto, `off` as 4:3
(`aspect` wins if both). Default: Auto, 16:9 on the browser played by touch (`touch_page()`).

## How
- `widescreen::view(size)`: the window's centred rect of the fixed ratio (all of it at Auto, and in a portrait
  window so B112 can lay that out). `ASPECT` (atomic index) replaces `WIDE`.
- `hud_gamma::follow_window` sizes the scene/HUD images to the view and sets the composite camera's viewport to it;
  the camera's output clear is black, so the rest of the window is the bars. The 3D projection and every
  `Val::Percent` layout follow automatically; code that read the window's aspect/size (`window_aspect`, `unit_px`,
  the FOV setters, grid, ticker, legend, QR, mods page file input) reads the view's (`widescreen::aspect`/`view_size`).
- Pointer and touch hits (`pointer.rs`, `pause.rs screen_point`) map into the view (`in_view`).
- Live: the main menu stores the setting each frame (`fit_grid`); the pause menu's Settings in `after`.
  `--original` forces 4:3 for its match; `--4x3` = 4:3; new `--aspect NAME`.
- The old widescreen-off mode (HUD pinned to 4:3 + UI bar nodes) is gone: a 4:3 view does the same.

## Checked (screens in `context/b117/`)
- Auto vs main's binary at 1280×720: Settings screen pixel-identical; match shot differs in 22 px (animation timing).
- 1600×720 (20:9) at 16:9: Settings and match (score panels, markers) inside a 16:9 rect, black bars at the sides.
- 1280×720 at 4:3 (side bars), 21:9 (top/bottom bars), 16:10 (match, character select at 21:9).
- `tools/check.sh -p hst` passes; `cargo check --target wasm32-unknown-unknown` builds.

## Not verified / not 1:1
- Browser on a 2400×1080 touch viewport: not run (no browser driven in this unattended run); same code path as
  desktop except the CSS-px pointer size, which goes through the same `in_view`.
- B112 / B118 (portrait, two-player portrait) aren't done yet, so nothing to re-check; a portrait window ignores the
  fixed ratio and keeps today's layout.
- Pop-ups mid-rally at fixed ratios only checked via the score panel/markers in a still; they use the same
  `window_share`/`window_aspect` helpers.
- Front-end/presentation only: no gameplay or sim change, nothing to match against the original.
