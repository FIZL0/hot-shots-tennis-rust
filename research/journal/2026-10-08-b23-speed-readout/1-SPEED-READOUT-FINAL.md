# B23 — serve/smash speed readout, ace banners in play

## The original (scoreboard object, pointer 0x42d6c0)

- Trigger: each frame the shot counter (0x423060) changes with a hitter set (0x423058 ≠ −1), the stats object's
  flag +0x750 is set (0x3a40c0/0x3a4580); the scoreboard update (0x383fa0) starts the readout (0x3895d0) when the
  hitter's branch (0x423f80 record +0xdd: 0 serve, 4 smash) is 0 or 4. Restarts on every serve or smash.
- Start: on +0x534, alpha +0x538 = 128, stage +0x548 = 0, t +0x544 = 6; mph = (int)(km/h · 0.9 · 0.62137), km/h =
  |ball vel|·60·3600/1000 (0x375cb0) read the frame after the strike (= the launch velocity); digits +0x54c..,
  count +0x558; side +0x55c = hitter player's x < 0.
- Tick 0x3898c0 (runs the start frame too): stage 0 alpha (int)(128 − 128t/6), t−1, t<0 → stage 1 t 60; stage 1
  t−1, t<0 → stage 2 t 6; stage 2 alpha (int)(128t/6), t<0 → off. 75 ticks.
- Draw 0x389760 (`inpane_speed00`): "mph" UV (248,0,56,32) at (x0, 216), x0 = 40/64/88 for 1/2/3 digits, 576 on the
  right; digits 24×32 cells (u = d·24) leftwards from x0 − 24, ones first.
- Draw and tick skip while a vtable check (0x1e9ea0 +8) is true; not identified (pause/replay; the port has neither
  running during play).

## Verified

`research/b23_speed_rec.py 5 3000 context/b23` (slot 5): serve 71 mph left, smash 84 mph right, serve 87 right /
78 left; the digits come from the strike frame's velocity, timing as above. Unit test `speed_tests::readout` pins
both RAM velocities, alphas, length and layout. Port screenshots (`context/b23/port_speed.png`,
`port_banner.png`) next to `orig_speed.png`: same place and art. Autoplay fires Service Ace, Return Ace and Counter
banners and they draw.

## Port

`popups::Speed` (resource), `tick_speed` after `simulate`, `draw_speed`; branch from `Finish::prev`.
