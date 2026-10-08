# B36b: the ball's draw scale, outline and shadow alpha

Ball draw, 0x37a7a0 (the ball object at b):

- depth = view-space z of the ball (+0xe0) through the view matrix 0x1e7e30; t = tanf(fov·0.5·0.017453292), fov at 0x1e7d50.
- Model scale (`*(*(b+0x280))+0x120`) = +0x8d4 (2.0) × max(0.13·(depth·t), 1) when the match phase gm+0x55 > 1
  (2 serve, 3 rally, 4 point over); at 1 (change ends) just 2.0. The multiply order matters (1 ulp on 51 of 900 frames).
- Outline (+0x28c, a second ballshadow.mdl): only drawn at ball alpha (+0x970) 1, at alpha 1 × ball alpha; scale
  2·ballscale·clamp(0.4·(depth·t), 0.8, 1); matrix rows x = norm((pos−eye) × d), y = norm(d × x), z = d = norm(0x1e7d40)
  (the camera's down axis, `view.rot[1]` in the port), t = pos.
- Shadow (+0x288): alpha 0.7 × ball alpha. Ball alpha only drops in practice mode (0x3c13c0 / 0x3bd970), which the app
  doesn't have, so 1.
- Both discs are GS draws with fog. Alpha 0.7 → colour alpha 0x59 → TFX MODULATE (HIGHLIGHT2 only at 0x80), so the
  texture alpha is scaled; with HIGHLIGHT2 the port drew a solid black disc.

Ported: `shade::half_fov_tan` / `ball_scale` / `ball_outline` (hst-sim), test `ball_scale_and_outline_match_the_game`
on `context/fixtures/b36b_ball.bin` (900 lock-step frames, slot 5, `research/b36b_ball_rec.py`, 183 grown) — bit-exact.
`play/ball_shadow.rs` places them; courts without a sun-shade map take the match world's ground.

Screenshots (context/b36b): orig_510.png vs port3_11.png — shadow centre ≈0.4× grass, soft edge in both; ball with
dark rim in both (cmp3.png, cmp_ball.png). `research/b36b_shot.sh n out.png` takes an original shot n frames after slot 5.
