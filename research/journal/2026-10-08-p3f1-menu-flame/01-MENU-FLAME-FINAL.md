# The menus' flame

## What draws
All callers before the seed sit in three menu-overlay functions of one effect object (24 particles, sprite
`../data/menu/2d/f_s.tm2`, alt `Blue.tm2`; config block in the menu overlay: life 18 + `% 24`, alpha ramp from 14
frames left, float speeds/decays not draw-relevant):
- setup (made by the screen's constructor, once): per particle `% 3 != 0` → spawn, else a second draw `& 7 == 0` → spawn.
- spawn: heading `% 360` (integer degrees), life `18 + % 24`, spin `% 7 − 3`.
- frame: count dead first; dead particle: < 13 dead → `& 7 == 0` spawns; ≥ 13 → `% 3 != 0` spawns, else `& 7 == 0`.
  Then each live one (a new one too): life −1 (0 → dead), else the heading steers toward 270:
  0..90 −(a+90)>>2 · 91..180 +(270−a)>>2 · 181..225 +(270−a)/6 · 226..269 +max(1, (270−a)/6·(`%200`+100)/100) ·
  270 ±1 (`& 1`) · 271..315 −max(1, (a−270)/6·(`%200`+100)/100) · 316..359 −(a−270)/6; wrapped to 0..359.
  All draws are `rand() >> 8`.

In the logs the setup lands at P1's character pick (vsync 5052); frames run from ~1 s later until the select closes
(c1 5115..5332, c2 ..5384); the count is all timing (4563 vs 5794 calls for the same picks).

## Port and test
`rng::MenuFlame::{new, frame}` keeps only what the draws read (alive, life, heading). `tests/rng.rs`
`menu_flame_draws_like_the_game` recovers rand() from boot, runs the setup, splits the log into game frames where
the particle pointer (s0) falls back (frames straddle vsyncs: the vsync counter ticks mid-frame, so vsync groups
don't split them), and requires every port frame to end exactly on a game frame's end and the last on the seed:
c1 4563 calls in 190 frames, c2 5794 in 242, all exact.

## Gaps
- The app has no character select, so main.rs keeps the clock-picked count (comment points at `MenuFlame`).
- The flame's drawing (motion, spin, alpha, sprites, which screen element it sits on) is not ported → P3f1a.
