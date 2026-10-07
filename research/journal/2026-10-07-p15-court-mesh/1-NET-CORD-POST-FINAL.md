# P15 — court collision mesh: net cord, posts, net stops (done)

P0c3/P0c4 had ported the mesh sweep (12498 live frames of net_s05 bit-exact on court 10: net 26 ×5, cord 2 ×3) but
no post hits, no serve lets, and the app still used the flat net without `--stage`.

## What changed
- App: the live ball always steps through `step_world`; without `--stage` the world is court 10's (the same default
  as the umpire's chair). `Flight::step` (flat net) stays for stored-path tests.
- `tools/record_live.py --vel vsync,vx,vy,vz`: overwrites the *live* ball's velocity (`*(gm+0x88)`) once right after
  sampling that vsync and writes `<out>.bin.poke`. The old `trace_live.py --net` wrote `*(gm+0x98)` (the predictor),
  which is why its ball "went through the net" in the first net-hit session.
- `tests/live.rs`: `replay()` shared; new `aimed_net_cord_and_post_hits_match_the_game` replays every
  `context/live/p15/*.bin` (skipping the two frames after a poke) and requires post (22), cord (2) and net (26).

## Picking the shots
Slot 5 is deterministic: the sample at vsync 7864 (a rally shot) is byte-identical in net_s05.bin and in fresh
recordings; 7785 is the first serve's launch. Aiming by gravity alone fails (the game's drag and topspin dip drop
every ball into the net), so the velocities were chosen by stepping hst-sim from net_s05's state at that vsync over a
grid (search kept in `context/live/p15/aim_search.rs`, recipe `context/live/p15/run.sh a|b`, slot 5, 520/450 samples):

| file | vsync | velocity | game's contacts |
|---|---|---|---|
| rally_post_back | 7864 | -0.09200001,-0.09200001,-0.58 | post 22, then walls/ground 13/48 |
| rally_cord_near_post | 7864 | -0.08400001,-0.1,-0.58 | cord 2, dribbles over |
| rally_cord_over | 7864 | -0.056,-0.1,-0.54999995 | cord 2, dribbles over |
| serve_let | 7785 | -0.148,0,0.58 | cord 2 in the middle, over (let) |
| serve_cord_back | 7785 | -0.16000001,-0.072000004,0.31 | cord 2 twice, stays back |
| serve_post | 7785 | 0.068,-0.08400001,0.25 | post 22 |
| net_low | 7864 | (old --aim run) | net 26, stopped |

All 1683 compared frames bit-exact, contact frames included. Court 10 layout probed with sweeps: posts (22) at
|x|≈5.95–6.05 up to 1.07 m, cord (2) along the top sagging to ~0.9 m at the centre strap, net (26) below.

## Left
- Courts other than 10 are built (every_court_builds_its_world) but only court 10 is checked against the game;
  the disc folder ↔ physics court index mapping is P15a.
- Let *rules* (calling the serve let) belong to P6.
