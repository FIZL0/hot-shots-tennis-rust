# B146 — menu hand clipped when it points up (FINAL)

## Cause
The hand's camera (`play/main_menu/cursor.rs`) rendered only the 64×32 layout box of the fingertip-right pose.
Once per loop `cursor.ani` lifts the wrist and points the finger up, and the hand moves up and to the left. It
left that box and was cut off.

## Fix
- `cursor::PAD = 3`: the camera's ortho box and the render image are 3× the old one in both directions, with the same centre
  (`VIEW` unchanged, 768×384 image, still 4× the layout).
- `main_menu.rs` draw: a `Tex::Cursor` quad showing the 3D image is grown by `PAD` around its centre. The hand keeps its
  on-screen size and anchor, and the four layout call sites didn't change. It goes through `widescreen::place` as
  before, so it works at any aspect ratio.

## Check
- With the camera's clear colour tinted for debugging, the padded box shows exactly where expected (`context/b146/dbg/f0.png`). The
  hand is the same size and in the same place as before.
- 80 shots 50 ms apart (4.0–8.0 s, `context/b146/dense/`), with grey-pixel bounding boxes. The raise (frames 27–29) peaks
  50 px (2048×1152 shot) above the old box's top. The 3× box leaves 32 px above it at the peak. A 2× box (41 px margin) would
  still have clipped it, so 3 is the smallest whole factor that fits.
- Final shots of the raise at 16:9 and 4:3: `context/b146/final/`, strip `context/b146/final_raise43.png`. The hand is drawn whole.

## Not verified / not 1:1
- No new PCSX2 screenshot of the original's raised hand. B135's 7 Options-screen shots (`context/b146/orig/`, copied
  from s4's `context/b135/orig`) all show the fingertip-right pose. Catching the brief raise with single
  `tools/screenshot.sh` calls is a matter of luck. The task says the original draws it uncropped, and the port now does too.
- The peak was sampled at 20 Hz, so the true peak could sit a few px higher. The 3× box has about 30 px to spare.
- Only the main menu was shot. The other `Tex::Cursor` quads (mode/select screens) go through the same draw path.
