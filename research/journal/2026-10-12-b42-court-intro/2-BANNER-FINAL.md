# B42b — "Start the Match!" banner (FINAL)

Ported from the court-presentation overlay's draw and tick (the branch taken once the camera shot is done and the
presentation's "banner" flag is set), `play/intro.rs`:

- **When**: the intro's last 92 ticks (`BANNER`). Every court's recorded intro length (`Rngs::intro_ticks`) is the
  camera shot's end (record times + 2 frames per cut) plus 92 = the model's 90 frames + 2. The flyover camera now
  holds until the banner starts (it used to hand over at `intro_frames`, 2·cuts frames early).
- **Model**: INPANE `courtpreview/i_gamestart_00` (ANI 7200 / 80 = 90 frames, MOR + MTA tracks), on the call
  models' overlay camera (CALL_LAYER), scale 3.3, distance 10, turned π like the call models, centred on screen
  point (321, 203) of the 640×448 picture. Started on the banner's first tick, one frame a tick, held at the end.
- **Pill + words** (`inpane_start00`–`02`): pill at y 240 (outer line pieces stretched 168 wide, middle 264), the
  sets/games digits (white outline copy, then tinted 127,59,59 / 127,79,18), "Sets"/"Games" at y 248, "Set" + "1"
  at y 296. Sets digit = 2·sets − 1 (best of), games digit = games to win; the game's "Set" digit is always 1 here.
  Alpha 0 → 120 by 8 a tick (15), held 60, back down (15).
- **Aspect ratio**: the middle pieces sit on the centred 4:3 screen; the two outer line pieces stretch out to the
  window's edges in widescreen (to the 4:3 edges with widescreen off).
- Tests: `play::intro::tests` (fade timeline, digit cells, pill width).

Side by side (court 8, 16:9 `context/b42b/mine/direct.png` at 5.7 s vs the original `context/b42/run1/020.png`):
same model, pill, digits and positions by eye.

## Not verified / not 1:1
- The banner's alpha/stage timeline and the model's start tick weren't recorded over PINE: the recorder
  (`research/b42b_banner.py`) needs a route from copy 5's slot 2 (main menu) through character and court select to
  a match, which I didn't script. Taken from the decompile; it's within a tick of it by construction, not measured.
- The scoreboard's own copy of the banner (y 176, started by the same message, drawn only after the presentation
  flag is cleared at the first point) wasn't drawn: it's unclear whether it ever shows (its sprites' alpha is the one
  the intro left at 0), and no recording was made to check.
- Draw order: the original draws the model before the pill; our UI nodes always go over the 3D overlay, which is the
  same order here.
- Only checked at 16:9 and by construction for other aspect ratios (no narrow-window screenshot).
- The HUD markers and the score panel still show during the banner (B42a hides them).
