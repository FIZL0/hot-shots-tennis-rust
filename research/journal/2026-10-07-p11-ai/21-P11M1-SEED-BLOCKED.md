# 21 — P11m1 AI seed: blocked

The chain (decomp):
- libc `rand()` 0x11f840: a 64-bit LCG at `*(0x1b80f0)+0xa8`, `x = x·0x5851f42d4c957f2d + 1`, returns
  `(x >> 32) & 0x7fffffff`.
- The main generator gm+0x80 is an MT19937 object (0x9d0 bytes, state +4.., index +0x9c4), made with seed 1
  (reseeded with `rand()` when 0x1bb750 is set) and reseeded with 0x42303c (= a `rand()` drawn at match setup) in
  0x324b50.
- Each AI object (player constructor 0x345510 → 0x3c8680 singles / 0x3ce410 doubles, also a human with a CPU
  partner in doubles) runs 0x35edb0, which reseeds the AI MT 0x427130 with one draw of the main MT. Its static
  seed (0x422390) is 1.

Checked on save states (`scratchpad/seedprobe.py`, pine reads of 0x42303c, gm+0x80's MT, 0x427130 and libc):
- slot 5: the main MT is exactly `Mt::new(0x42303c = 59222445)` after 4 draws; 59222445 is libc's output 1297
  calls back (the LCG run backwards). Slot 8: 0x42303c is 0, the main MT isn't any small-seed init.
- The AI MT matches no seed tried, up to 20000 twists: main draws 0..200 of that MT, of Mt(1) (0..100000 at 1500
  twists), of Mt(libc outputs 1200..1500 back), nor libc outputs 0..5000 back directly.

So the AI objects were seeded from the main MT before its match-setup reseed (or by a path not found). Their draw
index depends on everything since boot. Checking the app's seed needs a recording starting before a match's setup
(menus to a new match). No save state starts there: slots 3/4/5/8 are all mid-match.

The app keeps `Game::ai_mt` on one fixed seed.
