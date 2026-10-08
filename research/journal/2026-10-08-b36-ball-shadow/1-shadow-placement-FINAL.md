# B36 — ball shadow

## What the original does
- Ball draw 0x37a7a0, ground/light update 0x379e30. Ball object `*(*(0x422f80)+0x88)`: +0xe0 pos, +0x8d0 radius 0.064,
  +0x8d4 scale 2.0, +0x970 alpha, +0x8e0 ground normal, +0x930..0x96c shadow matrix, +0x288 shadow instance,
  +0x28c a second ballshadow.mdl instance used as the camera-facing outline billboard.
- Ground: on court (|x| ≤ 10.685, |z| ≤ 19.885) point (x,0,z), normal (0,−1,0); off court a ray 200 down against the
  court model; a miss keeps the matrix.
- Matrix rows: y = normalize(−n); x = normalize(cross(y, eye−point)) (e = (0,−1,0) when −n.y ≤ 0.1);
  z = normalize(cross(x, y)); translation point + n·0.005. Scale ball scale × 2.5 = 5. Row 2 × s, s = 1 for horizontal
  camera distance ≤ 10, else lerp(1, 3, clamp((d−10)/40)). Alpha 0.7 × ball alpha.
- ballshadow.mdl: 0.05 quad in xz, prim 0x70 (tex, fog, ABE), TEST 25; 64×64 PSMT4 black soft disc, CLUT alpha 0..128.
- Camera eye 0x1e7d20, full fov 0x1e7d50 (20°).
- Ball model scale: 2.0 × max(0.13·depth·tan(fov/2), 1) when gm+0x55 > 1 (3 in slot 5). Outline billboard scale
  2·ballscale·clamp(0.4·depth·tan, 0.8, 1), drawn only at ball alpha 1. Both left as B36b.

## Port
- `hst_sim::shade::{ground, ball_shadow, ball_shadow_stretch}`; `ball_height` now uses `ground`.
- Test `shade.rs ball_shadow_matches_the_game`: `context/fixtures/b36_shadow.bin` from `research/b36_shadow_rec.py`
  (HST_LOCKSTEP=1, slot 5, 1500 frames): 1499 bit-exact (137 off court); the first frame has no shadow yet.
- `crates/hst/src/play/ball_shadow.rs` places it after `draw`, 5×, blended at 0.7.

## Visual
`context/b36/cmp1.png` (orig slot 5 vs `--shot`): same ellipse shape, size and spot. The port's centre is a bit darker
(~30 vs ~55 on ~120 grass); the original's has fog and the PS2 output blur — B36b.
