# B186 Show FPS

- `graphics.rs`: `Graphics::show_fps` (settings.txt `show_fps = on|off`, flag `--show-fps`, off by default, kept
  across preset changes like `windowed`). `FrameTimeDiagnosticsPlugin` is added by `graphics::plugin`;
  `fps_counter` spawns/despawns a UI text at the window's top-right (1% in) on the default UI camera, so it shows
  over menus and matches; its size is 16/448 of the 4:3 screen's height (window height, or width × 3/4 when taller
  than 4:3), so it scales uniformly with the HUD. Applied live through the existing SaveSettings → `Graphics` path.
- Settings → Graphics: "Show FPS" row after Frame Limit; the screen now has ten rows, pitch 39.5 → 35.5 (Settings
  screen keeps 39.5). Web keeps everything but the Display row.
- Checked with `--shot` (context/b186/): Graphics screen 1280×720, main menu 600×1000, `--play --show-fps` match.

## Not verified / not 1:1
- Remaster-only setting, nothing to match in the original.
- Web build not run (no wasm-specific code added; the diagnostics plugin is cross-platform).
