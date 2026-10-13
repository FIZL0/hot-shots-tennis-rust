# B147 — menu hand wags only when idle (FINAL)

## Finding (menu overlay decompile)
The cursor object's init loads three motions from a name table: `cursor.ani` (index 0, looped), `cursor_ok.ani`
(1, once), `cursor_op.ani` (2, looped). Every menu screen moves the hand with the same pair of calls: start a glide
to the new spot (16 px a frame, the glide object's speed) and "play motion n from frame 0" — and all 17 call sites
pass n = 0. So each cursor move restarts `cursor.ani` at frame 0; there is no idle timer and no blend.

The idle look comes from the clip itself (1200 frames at 80 ticks/frame): the fingers barely move for the first
~60 frames, then the wag runs frames 60–120, rests, wags again at 240, 420, 600, 810, and a longer fidget from 900.
A hand that keeps moving keeps restarting, so it holds still; ~1 s after the last move it starts to wag.

## Port (`play/main_menu/cursor.rs` `tick`)
`tick` collects the visible UI nodes showing the hand image and their position; when that set changes (a move, a
new screen, a slide-in) it restarts the effect at frame 0. No change to `main_menu.rs`.

Checked: main menu, ArrowDown at 6 s, shots 0.3–2.5 s after (`context/b147/run1/crops.png`): still up to 0.9 s,
fingers wagging at 1.3–1.7 s.

## Not verified / not 1:1
- Not compared with PCSX2 screenshots: the decompile is unambiguous (every move restarts frame 0) and the menus
  aren't 1:1, so the side-by-side was skipped.
- The original glides the hand to its new spot at 16 px/frame; ours jumps (except during screen slides). Not part of
  this task; menus don't have to match.
- `cursor_ok.ani` / `cursor_op.ani` are still not played (no call site in the menu overlay passes 1 or 2 here).
- Window resizes also restart the clip (node positions change); harmless.
