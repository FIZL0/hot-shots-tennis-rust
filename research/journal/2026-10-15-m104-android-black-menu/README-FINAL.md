# M104: Android main menu "black" on the emulator (2026-10-15)

## Finding: no bug, a slow first load
The menu draws on the emulator at current main (3f317000). It isn't a regression since the 67039f02 prototype, and
it isn't a SwiftShader rendering failure. The first mode's loading screen (`mode.rs`, a plain black cover) stays up
until no pipeline is still compiling, or until its 600-frame cap. SwiftShader runs at ~1–2 frames/s while it
compiles the menu's pipelines, so the cover is up for **~5–6 minutes** after launch. M99 took its screencaps inside
that window and saw only black. The ~1 frame/s SurfaceFlinger rate it logged is the cover being presented.
M101's journal had already noted "the first menu load on SwiftShader takes ~7.5 min (600 frames)".

So no bisect and no code change.

## Verified (emulator hst34, API 34, x86_64, SwiftShader; own `-read-only -no-window -port 5640` instance)
- `HST_ABIS=x86_64 tools/apk.sh` at 3f317000, installed, launched with the adb-pushed ISO (no pack). Screencap every
  minute (mean of the image): black until 11:29:57. `loading screen down after 600 frames` was logged at 11:30:09,
  5 min 24 s after the launch at 11:24:45. From the next screencap on: the Main Menu (Local / Online / Mods /
  Settings, the Local card with 1P Keyboard, "Disc: SCUS-97610" top right), correctly laid out in landscape
  2400×1080. Screenshots are in `context/m104/` (t5–t10.png).
- The same with the imported `disc.pack` (the M99 route, pushed next to the ISO, so the pack wins): launched
  11:36:31, `loading screen down after 600 frames` at 11:42:37, then the same menu (`context/m104/pack.png`).
- How to check: wait for `adb logcat | grep "loading screen down"` before taking a screencap.

## Not verified / not 1:1
- **Real phone**: none attached to this machine (`adb devices` lists none), so this goes to plan/HUMAN.md. A phone GPU
  compiles pipelines far faster, so the cover should go away in seconds there, but that isn't proved.
- arm64 not built (only the emulator's x86_64 ABI).
- The black cover on a slow device looks like a hang: it shows nothing for minutes. Not changed here (the front-end
  is ours to design; a spinner or progress text on the first cover would be a separate task if wanted).
- Front-end is ours (PLAN.md): no comparison with the original needed.
