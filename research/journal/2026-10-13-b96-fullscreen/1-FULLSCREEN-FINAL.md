# B96 — launch in borderless fullscreen

User 2026-10-11: start borderless fullscreen on the current monitor by default, keep a windowed option, `--shot` keeps
its floating window.

## What changed
- `graphics.rs`: `Graphics.windowed` (settings.txt `windowed = on|off`, match flag `--windowed`), `window_mode()` →
  `BorderlessFullscreen(MonitorSelection::Current)` unless windowed or `KEEP_WINDOWED`; the existing `window` system
  applies it live when Graphics changes. Presets ignore it (choosing a preset keeps it, and a windowed Default still
  reads "Default").
- `main.rs`: the window opens in `gfx.window_mode()`. For the menu, settings.txt's graphics lines are read before the
  window is made, so a windowed setting doesn't start fullscreen and then shrink. `KEEP_WINDOWED` = `--windowed` on the
  command line, `--shot`/HST_BG runs, or the web build (browsers need a click for fullscreen). `--fullscreen` (and the
  Steam/Deck env check) is now redundant; `--fullscreen` is still accepted.
- Settings → Graphics: new "Display" row (Fullscreen / Window). Nine rows didn't fit at 42 px from y 66 above the
  description bar, so that screen's rows are now 39.5 px from y 46 (`GRAPHICS_TOP`/`GRAPHICS_PITCH`, shared with the
  click hit-test in `pause.rs`).

## Checked
- `hyprctl clients` on a live run (6 s each): plain launch → fullscreen 2 (Hyprland's fullscreen) at the monitor's
  size; `--windowed` → 0; settings.txt `windowed = on` → 0.
- `HST_MENU=graphics tools/shot.sh` (shot window stays floating 1280×720): `context/b96/graphics.png`, all nine rows
  between the tab and the description bar.
- Tests: preset/Display round trip in `main_menu.rs settings_round_trip`, `graphics.rs text_and_flags_round_trip`,
  row hit-test in `pause.rs rows_under_clicks`.

## Not verified / not 1:1
- Toggling Display live from the menu (fullscreen ↔ window) not driven by input in a live run; it goes through the
  same `window` system as the startup path, which was checked.
- Multi-monitor "current monitor" choice not tried (one monitor here); it's Bevy's `MonitorSelection::Current`.
- Front-end is ours (PLAN), so no comparison against the original.
