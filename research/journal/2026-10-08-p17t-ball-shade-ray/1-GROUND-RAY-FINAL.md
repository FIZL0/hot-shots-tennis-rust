# P17t — ball shade off the court: the ground ray

Checked against the P17l slot 5 recording (court 10, bot rally, 1300 frames: ball position and its model's light
scale, `research/p17t_ball_rec.py`) with the game's own map: `crates/hst-sim/tests/shade.rs`. 1299 frames are
bit-exact, all 137 off-court frames included (31 of those missed with height from y 0). The other frame is the
between-points reset: the ball is put at the origin after that frame's light update, so the light update keeps the
old scale.

## The ball's light update (0x379e30)
- It is skipped while 0x2eefe8 (pause/overlay) is set. On the court (the 0x379d80 limits, unless 0x410870) the
  ground point is (x, 0, z) and the normal is up.
- Off the court it calls 0x32f7a0(world, hit, start = ball pos, end = (x, y + 200, z, w), 0, 0, 0, 0). Ghidra
  shows 0x335ba0 with one argument, but a1–a3 pass straight through it. So it is really
  0x14ce70(court object, hit, start, end, 0, 0), then 0x15cf80: the ray against the court model only (no props).
  The hit record starts with t = FLT_MAX and the default material 0.
- 0x15cf80/0x15a250 mirror the sweep's 0x15c700/0x159630. The differences: the box is the bare segment box
  (0x159590), the triangle test is 0x130e90, and the shrink is 1.1t. Material 0x1580b0 (id 0 → rejected) and the
  node transform are shared. So `mesh::Object::ray` is the sweep loop with radius 0 and `ray_triangle`.
- When the ray misses, nothing is written (shadow normal, light scale): the last scale stays.
- height = −(ball y − hit y) (FPU), then `shade::ball` as before, with the lookup at the ball's own (x, z).

## Ray–triangle 0x130e90 (verts, hit, start, end)
- d = end − start (FPU). Edges (v0,v2), (v1,v0), (v2,v1): normalize(d × (a − b)) · (b − start) < −0.0005 → miss
  (VU subs, VU0 normalize/dot).
- n = (v1 − v0) × (v2 − v1), unnormalised (FPU subs, VU cross). de = n·(end − v0): must be ≤ 0. ds = n·(start − v0):
  ds < −0 → miss.
- t = ds / (ds − de) (FPU) if de < ds, at most 1; otherwise 1. Rejected unless t < hit.t.
- centre = lerp(end, start, t) (0x125c48: end·t + start·(1 − t)), w 1; point = centre; normal = normalize(n).

## Port
- `hst_sim::mesh::{ray_triangle, Object::ray}`, `hst_sim::shade::ball_height`. `hst::shade` keeps the court's
  collision world in `Shade` and skips the update on a miss.
- The 0x410870 flag (no arena limits) is treated as clear, like `judge` does.
