# B42d — court intro on other courts and modes (FINAL)

## What phase 0 waits for (decompile of the intro phase's tick)
- Practice (players = 1): ends on its own timer and the streamed sound; no plate/banner overlay (the overlay's
  create message 6 only builds it with players > 1).
- Singles **and** doubles (players 2 and 4) take the same path: wait for the camera shot to end (plus, when a
  certain mode flag is set, a check on the camera handler), then set the presentation's banner flag (mgr+0xd94 = 1);
  from then on phase 0 waits for +0xd95, which the overlay's tick sets once the "Start the Match!" model's
  animation is done; then phase 2. So the "doubles presentation flags" in `plan/B42.md` are the plate→banner
  flags of B42a/B42b and apply to singles too. Nothing doubles-only in the wait.
- The only doubles-only step (players ≥ 3): once, on the intro's first frame, it fills the four characters' sound
  banks (a voice table setup, gm+0x9c). Audio, not a presentation; no effect on the intro's length.
- Length per mode: already proven equal in singles and doubles for all 11 courts by `tests/rng.rs`
  (`intro_ticks_like_the_game`, `intro_ticks_from_the_disc`, P3f2/P3f6 recordings). No port change needed.

## Recordings (`research/b42_intro_camera.py`, doubles from slot 2, `context/b42d/cam_cNN.bin`)
- Courts 1, 2, 3, 4, 5, 6, 7, 9, 11 (plus court 8 from B42). The court-select steps aren't reliable (presses are
  dropped; steps 1→9, 2→2, 4→3, 5→7, 6→5 or 6, 7→1, 8→4, 9→6, 13→11): the recorder prints the court it got.
- `crates/hst-sim/tests/intro.rs` now checks every recorded court: channels 5–10, 14 within 1e-4 (angles mod 2π)
  and the eye within 1 mm + 2e-4 of its distance, every frame to the shot's end. Courts 3, 4, 5, 7, 8, 9: eye error
  ~0. Orbit courts 1, 2, 6, 11: a few frames up to ~7 mm off at 40–55 m (court 1 frame 76 as the eye's z crosses
  0; court 2 frame 234 where the yaw wraps by 2π). The test's start is now the first sample on shot 107 (court 1's
  channel 8 starts at 0), and a sample may mix the new channels with last frame's eye (court 3 frame 1).

## Side by side (`context/b42d/orig_*` original, `context/b42d/mine/` ours, 1280×720)
- Court 1: flyover overhead pass (orig 013 vs `c1_1.5`), the low academy pass (017 vs `c1_3.5`), banner (020 vs
  `c1_5.0`): same shots, plate, rows and banner.
- Court 11: the orbit over the pond (014 vs `c11_1.5`), banner (020 vs `c11_5.0`): same.
- Court 5 (`context/b42d/c5_sheet.png` vs `c5_3.5`): same stand pass; the crowd's CLAP balloons show during the
  original's flyover too.

## Not verified / not 1:1
- Court 10 wasn't reached by the court-select route (dropped presses); its camera is checked only against its
  file. Its length is covered by the P3f2/P3f6 recordings.
- Singles intros weren't recorded camera-wise (every route here starts doubles); by the decompile the camera and
  overlay path is the same, and the lengths are recorded equal.
- Orbit courts: a few frames' eye is mm off the original (above); not chased (camera = presentation).
- Screenshots are at different frames (my `--shot-at` seconds vs the original's sampled frames), so compared by
  shot, not frame. Lens flare/sun glare in the original's court 1 low pass isn't in ours (not this task).
- The doubles sound-bank setup isn't ported here (audio; B42c / N3).
