# B27c — the computer server's first stick after its wait (FINAL)

- **Original** (rally sub-state 0, BASE 3d1320; NET 3d02c0 the same shape): after the shot-count and reaction-frame
  gates (B27b), it builds the doubles hand-over (+0xd6 every 30 frames when its own team hit last, +0xd4/+0xd5 when
  the partner has the ball, the walk back 3d5a30), then gates on the path count 35f0d0 (< 1: no stick). Then:
  - a contact search finds the ball (361760 on +0xd0 frames, 3d23f0, 3d5dd0 → 3613e0, or 360150/360910 per level):
    +0x70.. locked, sub-state 1, then the step 35c970 toward +0x70 with keep on (moves at once: serve_ai.bin 8800),
    or the swing start when +0x90 frames are out;
  - else the spot helpers (361af0 with 0.3, 3621d0, 361e70 with 0xb, 361eb0) with the step's keep off, so the stick
    is zeroed within ⅔ of a step; sub-state stays 0 (10339 moves this way, the contact search locks at 10340).
- **Port**: `bot` no longer breaks a computer server's follow-through on `ai_serve_stick` alone: past the wait it runs
  its rally goal and step (`bot_run`, `Body::toward`'s ⅔ zeroing) and `ai_serve_step` ends the follow-through only
  when that stick is non-zero; a zero stick keeps it playing out. The break-off frame itself still stands (the run
  starts the next frame, as a human's).
- **Gap → B27d**: the goal is the stand-in's (`intercept`, `ai_wait`), not the tree above, so the "a frame later"
  start (spot helper then contact lock) isn't reproduced.
- **Test**: `player.rs ai_serve_stick` unchanged and passing (the wait itself); the ⅔ rule is `Body::toward`'s (P7f).
