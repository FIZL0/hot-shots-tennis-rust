# P1 shot parameters in play: launch frame, side-angled serves, play wiring

## Done
- **Launch frame (`shot::launch_frame`)**: built the way the game builds it. The horizontal unit vector uses the PS2 FPU
  (`div(1, sqrt(madd(mul(dz,dz),dx,dx)))`). Then up = (0,1,0), side = normalize(up × dir), ahead = normalize(side × up),
  and the frame is pitched by rot_x(elevation) (helpers compute `in · R`). Velocity = row 2 × speed.
  The matrix matches +0x160 bit for bit.
- **Lookup on the FPU**: distance, axes, lerps, the frames madd, the near-net correction and the low/high bounds now use
  `ps2` ops, which chop. Native f32 left speeds 1–2 ulp off. Volleys are 17 of 17 bit for bit.
- **Side-angled launch (`shot::launch_turned`, slice serves)**:
  - The rows [frame0..2, d = target − hit (w = 1)] are yawed by rot_y(spin). The yaw constant is 1.0.
  - wind = (target − (d' + hit)) / frames, with w = (target.w − 1) / frames.
  - vel = the yawed row 2 × speed.
  - The flight frame = rot_y(±side) · rot_z(±π/2) · frame, signed by spin, after which spin is made positive. A lefty
    negates spin when side ≠ 0.
  - frames = table + 1, ±1 for class 0 near the 2.0575 line.
  - Serves now match velocity, frame, wind, frames and spin bit for bit, slices and lefties included (tests/serve.rs).
- **play.rs**:
  - `strike` uses `serve::launch` / `rally_launch` for velocity, flight frame, wind and curve frames.
  - Serve spin and side come from the character record (`serve_tables`).
  - Smash spin comes from the record (`smash_spins`), not `5f32.to_radians()`.

## Left open
- **Strokes** are not bit-exact: the closest launches are 7 of 11 with elevation × the mis-hit scale (1, .95, .9, .85, .8),
  so the inputs differ slightly. The scale is the mis-hit roll (grade +0x3ee8 == 4; volleys use 1 − (1 − x)/2.2),
  which is **P3**.
- **Charged smash blend (normal ↔ charged)**:
  - The class 3 blend (+0x60) is nonzero on recorded kind 0 smashes: −0.2, −0.333, −0.45 (match_s05) and −0.04 to −0.15
    (human_smash_s04). Kind 1 is always 0.
  - When nonzero, the game runs a second lookup on the mode 1 record and its variant table, with the target offset.
    It then lerps elevation, speed, frames and fields 7–12 by |blend / variant weight| (clamped).
  - Every smash still launches within 2e-5 under the existing smash tests. The blend input comes from the smash charge,
    so port it together with **P3**.
- **The two MT19937 draws** at launch (+0x258/+0x25c, `utof(u) × 2.3283064e-10`) only feed the random-bounce option
  (**P21**).
- **Curve, bend and turn** for rally shots are left to **P6**; `rally_launch` passes bend 0.
- `context/shots_s05b` was never recorded; the fixtures in `context/fixtures` stood in for it.
