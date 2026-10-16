# B419 — main menu below the touch pad's top row (FINAL)

The phone apps' on-screen pad (`play/touch_pad.rs`) puts L1, the chat bubble, Select, Start and R1 along the top of
the window; they covered the menus' title bars and tabs.

## What changed
- `touch_pad`: `Layout::top()` = the top row's foot + the same 0.3 u margin it keeps above it (t + 1.3 u);
  `drive` publishes it as a share of the window height in `TOP_ROW` (0 while the pad is hidden, e.g. a real
  controller is connected), read with `touch_pad::top_row()`.
- `widescreen`: `TOP_GAP` / `set_top_gap`: `view` keeps below that share of the window (`view_below`) at every
  Aspect Ratio — Auto (safe rect's top pushed down), fixed ratios (largest rect in what's left, centred), Stretch,
  portrait. Each mode's `widescreen::plugin` clears it at Startup, so a match keeps the whole view.
- `main_menu`: `touch_gap` copies `top_row()` into the gap every frame. Since every menu layout, the taps
  (`pointer`), the 3D previews, the legend and the composite's viewport (`hud_gamma`) already go through `view`,
  all screens move down together and scale uniformly (nothing stretched); the strip above is black like the bars.

## Checked
- Tests: `touch_pad::the_menus_gap_clears_the_top_row` (gap below every top-row touch area, above face buttons and
  stick, landscape phone / 16:10 / portrait), `widescreen::views_in_the_safe_area` (gap at Auto, Stretch, 4:3–21:9,
  a deeper safe inset wins).
- `--shot`s with `HST_TOUCH=1` (context/b419/): main menu at 852×393 (vs. without the pad), character select at
  852×393, settings at 1024×768: menus start right under the row, nothing covered at the top.

## Not verified / not 1:1
- Not run on a real iPhone/Android device (no device in an unattended run); checked with `HST_TOUCH=1` shots at
  phone-sized windows, which use the same layout code.
- Remaster-only UI, no original to compare against.
- The bottom face buttons and stick still overlap the menu's lower corners as before — out of this task's scope.
