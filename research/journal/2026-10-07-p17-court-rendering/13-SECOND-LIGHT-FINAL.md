# P17s2 — the second light and the sun glare (2026-10-08)

**Question.** VU1 lights with a second directional light (qw28, the colour matrix's second row) that the port left
out. The players also have a third factor and a fog value of their own, and 138410 dims the light near the sun.
What are they, and do they match a GS dump?

**Answer: ported. The court, the models and the players each have a second light. Single-bone character vertices and
static court models match slot 4's dump with it.**

**The original.**

- **138410(block, params)** writes the light block:
  - RGB goes to +0x60, [ambient, light, third] to +0x70/+0x74/+0x78, and the directions to +0x80/+0x90.
  - +0xa0 = normalize(camera forward +0x240 + L).
  - The colour matrix at +0x420 has the rows RGB×light, RGB×third, 0 and RGB×ambient.
  - VU1: FTOI0(vc·(109·N'.x + 110·N'.y + 112) + 111·q), with N' = max(M·n, 0), so each light is clamped on its own.
- **The glare.** It applies only when the block's +0xf0 flag is set and DAT_001bb538 is set (weather < 2, from
  33a060). With d = dot(+0x240, +0x80) and −d > 0.8, x = (−d − 0.8)/0.2, so ambient ×= 1 − x/2 and
  light ×= 1 − 0.6x. The third light is not dimmed. 138a30 (the camera update) reruns 138410 every frame.
- **Which blocks have the flag.**
  - The court block (0x1e7d10) has it: 32d4e0 sets it.
  - The players copy the court block, so they have it.
  - The sky fog block (32f950 +0x170) does not; the cloud and background blocks do.
- **The court (32e190, 32e5c0).**
  - Direction 2 = exactly −sun.
  - third = the envir light row's +0x14. Unlike the light (+0x10), it is not × the weather's scale. Rain greys the
    RGB.
  - The models' block uses +0x20, or +0x14 where it is 0.
- **The players (3484d0).**
  - Direction 2 = (0, 0, −sign(+0x12b0)). +0x12b0 is +1 on the −z half.
  - The factors are the GAME table's [ambient, light, third, fog].
  - 1383d0 copies the court's main fog with +0x10c (near F) = table[3]. If z1 ≤ z0, z1 = z0 + 1.

**Checks.** The slot 4 capture is court 4 doubles, in `context/p17s2/`: s4.gs, s4.ram and s4.png, captured with
`research/p17s2_capture.py`. In RAM:

| Block | Factors (ambient, light, third) |
|---|---|
| Court | (0.7, 0.5, 0.02), RGB 1 |
| Models | (0.8, 1.0, 0.08) |
| Players | (0.75, 0.35, 0.1), fog near 255 |

These match envir_c04.dat's season 0 row and the table. The players' L2 is (0,0,−1) at end +1 and (0,0,1) at end −1.

`research/p17s2_check.py` fits one rotation per node to the single-bone vertices drawn in the dump. It compares
⌊vc·mat·(A + L·max(−Rn·l1,0) + T·max(−Rn·l2,0))⌋ with T and with T = 0.

| Model | Exact with the second light | Exact with T = 0 |
|---|---|---|
| pc03 (end +1, L2 (0,0,−1)) | 119/121 | 58/121 |
| pc11 (end −1, L2 (0,0,1)) | 32/32 | 5/32 |
| court model `ry` (l2 = −sun) | 202/204 | 63/204 |
| court model `shinpandai` (l2 = −sun) | 261/262 | 186/262 |

For pc11, the wrong direction (0,0,−1) gives 19/32.

**The glare is not checked against a dump.** The match camera is far from the sun: in slot 4, −d = −0.035.

**Port.**

- `gs::court_light`, `gs::model_light` and `gs::player_light` return (colour, ambient, second).
  `gs::player_light2(game z)` gives the players' direction.
- `GsUniform` gains `light2_dir`, `light2_color` and `glare`.
- gs.wgsl adds the second diffuse term and dims ambient and light by the glare x, using the view's forward.
- `weather::apply` sets the court's (−sun) second light and glare, which is off for skies and in rain.
- `shade::light` sets the models' (−sun) and the players' (by the side of the court they stand on). It re-lights when
  a player changes ends. The players' fog near F is their table value.
- Test: `gs.rs second_light`.

**Gaps.**

- **P17s4.** In the same dump, the court ground (`park_h01`), the net (`znet`, `netmoto`) and most of `house` and
  `light4` don't fit A + L·d + T·d₂ with or without the second light. The ground's GS/(vc·mat) median is 1.39, above
  ambient + light = 1.2. The net's is a near-constant 1.79.
