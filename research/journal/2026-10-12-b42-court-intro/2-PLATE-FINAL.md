# B42a — title plate and ✕ Skip

- Recorded the plate object per match frame on courts 8 (no skip), 4 (✕ at 100, ✕ at 320 in the banner) and 5
  (✕ at 5) with `research/b42a_plate_rec.py` (`context/b42a/*.txt`); findings in `plan/B42.md`.
- Draw (decompiled): a 640×80 bar from `CourtPreview_00` (176,48,64,80) at alpha 64; the court's
  `CourtPreview_nameNN` at (32,20); the underline in three pieces (16/40/480, y 46); the surface label tinted
  (grass 41,120,25 · clay 114,72,31 · hard 26,100,127); two icon + 4-ball rows at x 192 and 352 (bright 184 up
  to the court's rating, dark 208 after); after the fade, `info.tm2`'s ✕ (and its glow on alternate 15-frame
  halves) at (512,400) and "Skip" at (544,400).
- Fade: name alpha 128 − c·128/15, c = 16 − frame → recorded 0, 9, 18 … 120, 128 (frame 16); the test checks it.
- Port: `play/intro.rs` `plate` (quads), `draw_plate` (UI slots; bar spans the window, plate kept from the left
  edge as a group, ✕ Skip pinned right), `hide_hud` (panel slots, balloons, markers hidden while the intro runs),
  skip now ends only the flyover and leaves 92 ticks (`SKIPPED`). `exe::court_plates` reads the rows.
- Checked by eye: `context/b42a/mine/f95.png` (1280×720) beside the original's
  `s5/context/b42/run1/014.png` (court 8, frame ~95): same text, tint, rows, icons, Skip.

## Not verified / not 1:1
- The original keeps the plate 2 frames past the flyover (through the cut); ours hides it with the flyover.
- Which of the object's update/draw runs first in a frame: the fade is mapped so frame 1 draws alpha 0 as
  recorded, but one-frame offsets wouldn't show in the RAM log.
- The draw's gates (a global flag, a second flag and the player count ≥ 2) were all pass in every recording;
  ours always draws. Their other states weren't reached.
- Surface tints are constants in the code (taken from the overlay's init code, which loads them as immediates,
  not a data table `exe` could read).
- Only courts 4, 5 and 8 recorded; the rows of the other courts come from the table, not screenshots.
- The 92-tick rest after ✕ shows only the match camera; the banner itself is B42b. 92 measured on courts 4 and 5.
- Widescreen placement (the plate kept from the left edge, Skip from the right) is ours; the original is 4:3.
