# B42 iteration 1 — the intro camera flyover

## Original
- Captured court 8 from boot to the first serve without a monitor: `tools/screenshot.sh` now falls back to the
  copy's pad-bound PCSX2 Screenshot hotkey (L3+R3, set in the copy's PCSX2.ini) when hyprctl can't reach a window.
  `research/b42_intro_shots.py` takes the screenshots; `research/b42_intro_camera.py` records the shot camera every
  frame (`context/b42/cam_c08.bin`, 681 frames).
- Phase 0 is the intro. While the "Now Loading" tips screen is up, the camera script holds; it steps with the match
  frame counter. The court title plate and "✕ Skip" show during the flyover. After the script, the match camera
  shows "Start the Match!" / "1 Sets 4 Games" / "Set 1" until the intro's tick count (`Rngs::intro_ticks`).
- Script: each court's `cam_cNN_00.dat` → the game's camera-shot script (its compiler and orbit conversion are
  in `plan/B42.md`), run as camera shot 107 (subject = court centre). A cut lands two frames after the previous
  move ends. The roll channel is not in the shot's view rows, but the picture is tilted by it (screenshot 016).

## Port
- `hst_sim::cutaway`: `intro_script` (file → script, orbit segments converted to framing angles + distance),
  `intro_frames`, `Shot::intro`, `Shot::court` (the only reference frame), `Shot::rolled` (the view turned by
  the roll channel).
- `play/intro.rs`: from PostStartup, loads the court's camera file. Before each frame's match ticks, it holds the
  match (same fixed-timestep freeze as the pause screen, re-applied every frame so closing the pause menu can't
  restart it) and runs its own 60 Hz clock. During the flyover it sets the camera transform and fov (rolled),
  before the backdrops follow the eye. Then the match camera holds until `intro_ticks`. ✕/Enter/Space/J skips.
  Off online, for `--shot` runs (unless `HST_INTRO=1`), with `HST_INTRO=0`, and headless. play.rs: two lines
  (`mod intro;`, `add_plugins`).
- Sim/RNG untouched: the flare ticks were already drawn at setup; no FixedUpdate runs during the intro.

## Checks
- `crates/hst-sim/tests/intro.rs`: on all 11 courts, every record's eye, roll and fov are reached at the end of
  its move (including the orbit segments on courts 1, 2, 6, 10, 11). Court 8's channels 5–10 and 14 and the eye
  follow the recording for every sample up to frame 300 (within 1e-4 relative / 1e-3 m).
- Remaster shots (`HST_INTRO=1 hst … --stage 8 --play --shot-at T`, `context/b42/mine/`) beside the original's
  (`context/b42/run1/`): the overhead pass, the rolled low pass (tilt matches the original's direction) and the
  hand-back to the match camera look the same.

## Not verified / not 1:1
- Courts other than 8 were not recorded in the original. Their scripts are only checked against their own files
  (poses reached), not against the running game. B42d records courts 1, 5 and 11.
- Roll: applied by eye (sign checked on one court 8 screenshot). Where the original applies it wasn't found.
- Skip: ✕ ends the whole intro, including the banner part. Not checked against the original's skip; B42a.
- The loading-screen hold before the flyover isn't reproduced: we have no loading screen in a match.
- Banner part: the match camera is shown, but its "Start the Match!" overlay isn't drawn yet (B42b). The court
  title plate and Skip prompt aren't drawn yet either (B42a), and the HUD panel and balloons still show.
- Music and sounds: unchanged (B42c). Doubles player presentation: not done (B42d). Online: intro off (B42e).
- The backdrop card can end inside the low shot's sky on court 8 (blue gap at the top left). It follows the eye,
  but its width was made for the match camera. Rendering: left as a note.
