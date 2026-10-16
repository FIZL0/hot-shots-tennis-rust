# B417 — Android APK hangs in the main menu

## What the logs say
Phone logs (user, 2026-10-15, `context/android/`; Adreno 830, SM-S931U):

- `hst-log.txt` (the hang, 19:13Z): the menu bakes the court previews (d02, d05, d13, d04, d03 at ~2 s), then
  `render stall 11607 ms … graph+submit 11603.6` at 14.6 s, `10016 ms` at 24.7 s, `2003 ms` at 26.7 s, then
  `Caught DeviceLost`. The CPU side (assets, prepare, encode) stays at a few ms; the time all goes into the queue
  submit, so the GPU itself hangs.
- The task suspected shadows=high and MSAA 4x, but the same log rules them out for the menu: every pipeline queued
  in the menu is `samples 1` (no `samples 4` line at all; the match log from B416 has 62), and the draw census at
  15 s and 25 s shows `shadow casters per view and cascade [[0, 0], …]` (no caster in any cascade). So neither the
  shadow pass nor MSAA was drawing anything when the GPU hung.
- What that build *did* use is Bevy's GPU-built indirect draws (`GPU preprocessing is fully supported on this
  device`, and nothing turned indirect off): one `bin_unpacking` compute dispatch per batch set and a
  multi-draw-indirect-count per batch, for every 3D camera, including the preview bake's offscreen ones.
- Timing: the hang log is from 19:13Z = 15:13 local. P48 (all 3D cameras get `NoIndirectDrawing`,
  `graphics.rs direct_draws`) merged at 15:18:58 local (`f0f32ff5`), so the hanging APK was built before it.
- `run2/hst-log.txt` (19:20Z, an APK with P48): the same phone goes through the menu (preview bake at 19:20:29 and
  again at 19:21:26) and into a match (`--shadows low --msaa 2x`, Phone preset) and plays to 19:22:19. The worst
  stall in the whole run is 749 ms and the worst `graph+submit` 14.4 ms. No DeviceLost. The user also reports
  (TODO note) that an APK from `c19cb239` (after P48) runs well and fixes the missing ball FX/outline, which was
  another Adreno indirect-path symptom.

## Conclusion
The menu hang came from the Adreno driver choking on Bevy's indirect-draw path. P48 replaced it with direct draws
for every `Camera3d`: `direct_draws` runs in `PostUpdate`, so a camera spawned that frame (the preview bake reuses
the main camera, the cursor and popup cameras spawn their own) is direct before its first extract. HEAD has it, so
no code change is needed for this task. The settings the task wanted bisected (shadows, MSAA) weren't active in the
menu at all.

## Not verified / not 1:1
- **shadows=high + MSAA 4x on the phone after P48**: not retested. No Android device on this machine (`adb devices`
  is empty), and the emulator's GPU isn't an Adreno, so it can't reproduce a driver hang. The menu doesn't use
  either setting (samples 1, no casters), so they matter for B416's match start more than here. Added to
  `plan/HUMAN.md`: start the current APK with shadows High + MSAA 4x, sit in the menu through the preview bake,
  then start a match, and send `hst-log.txt`.
- Front-end/presentation only; no gameplay or original-game comparison applies.
