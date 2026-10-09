# P23 graphics settings: MVP (user 2026-10-09: "just get the video settings done now MVP", fps targets later)

Done:
- `crates/hst/src/graphics.rs`: `Graphics` resource (frame limit 30/60/120/144/uncapped, vsync, render scale
  50/67/75/85/100 %, shadows off/low/high), presets Default / Steam Deck / Low, flags `--fps --vsync --scale --shadows`,
  settings.txt keys `frame_limit vsync render_scale shadows` (old `uncapped_fps = off` reads as vsync on). Applied
  live: present mode, frame limiter (sleep in `Last`), shadow map size + cascades (low = 1024, 2 cascades; high =
  Bevy's 2048, 4 = the look before), shadows off = `shadow_maps_enabled` false.
- Render scale: `hud_gamma`'s scene image is sized scale × window and the composite samples it filtered (exact
  texel centres at 100 %, so the default look is unchanged); the HUD stays at full size.
- Menu: Settings → Graphics screen (Preset, Resolution, Shadows, Frame Limit, VSync); "Uncapped Frame Rate" row
  replaced. First run with `SteamDeck=1` picks the Steam Deck preset.
- `HST_FPS=1` logs fps + slowest frame every 5 s. `tools/slow.sh`: weak-device run (2 cores, 120 % quota;
  `HST_SLOW_IO=1` adds microSD-like I/O limits via sudo systemd-run, untested here: needs a password).
  Desktop dGPU under tools/slow.sh, 4-CPU doubles court 1: ~160 fps, slowest frames ~19 ms.

Not done (P23 stays open): Deck/iGPU profiling and the 120/60 fps targets; anti-aliasing setting → P23b (FXAA
dropped by the user; MSAA toggle blanked the court, maybe only the 10 s shot timing, see P23b).
Pitfall: match `--shot`s at 10 s can still be on the loading screen for some flag sets: use `--shot-at 25`.
