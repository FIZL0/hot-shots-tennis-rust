# P3e6 — cheer-mark sprites (port: `play/npcs.rs` `draw_cheers`, `exe::Game::cheer_sprites`)

## Original (0x39ba30, the gallery manager's draw)
- Gate: court weather (+0x135) ∉ {2, 3}, run flag +0x1b61, and (court 5 or 0x2eefb0 == 0). 0x2eefb0 is never
  written in the game overlay and is 0 in every dump: the port leaves it out.
- Texture: both sprite batches (+0x678[kind], made in 0x39b5d0) use the shared effect sheet *0x43b1d0 + 0x6a4 =
  the 10th of 0x3a3160's list at 0x415860: `npc/clap.tm2` (AZUMA/C_EFF/EFFCT.XB0). Blend kind 0x1a.
- Per shown mark (court 5: slots from +0x9f0): c = pos − row2(cam→world) × 0.5; view depth d of c (0x1e7e30);
  h = d · tan(fov/2) (0x1e7d50 = 20°); size = t0 · max(t1·h, 1) · min(t2·h, 1) (the 1.0s are literals at
  0x413840/0x413848, checked in the disassembly); c.y −= t3 (y is down: the sprite hangs above the mark).
  Corners c − r − u, c + r − u, c − r, c + r with r = row0·size, u = row1·size (row1 points down, so the quad
  rises from c), UV (0,0) (1,0) (0,1) (1,1), RGB 0x80, A 0x80.
- A = 0 when t4 > 0 and 0x369530(1.0, ·, c) says hidden. That call's threshold is $f13 = t4 (the int arg is junk):
  hidden when view z ≤ 0.001 or 1361.1 (proj [0][0] = 240 / tan(fov/2)) / z ≥ t4, i.e. too near the camera
  (kind 0: z ≤ 4.5 m, kind 1: z ≤ 17 m).
- Table 0x4131e0 stride 0x14: kind 0 [0.3, 0.1, 0.4, 0.5, 300], kind 1 [0.75, 0.08, 0.4, 0, 80].

## Check (`research/p3e6_marks.py`, `research/p3e6_fit.py`)
Slot 3 (court 4, waiting for the serve): 7 marks poked into the manager (delay 10⁶ so they don't jump) at depths
10–40 m, kind 0 and kind 1, screenshot each vs none. Projecting the ported quads through the game's matrices
(GS 2048 centre, a 640×224 field): every visible sprite's blob lies inside its quad, x spans to ~2 px, y to the
texture's transparent top rows; the 10 m mark is hidden for kind 1 and shown for kind 0, as predicted.
The port on court 5 (`--stage 5 --play --shot`) draws the 93 marks; none fall in the game's 4:3 serve view (one
shows at the port's wider window edge).

No court-5 screenshot of the original: no save reaches court 5 (court select isn't automated).
