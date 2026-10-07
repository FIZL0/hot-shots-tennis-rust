# P17g — shadows

The game has three kinds of shadow:

1. **Projected shadows.** These are used on day matches. The tree, umbrella and player shadows at court 10 come
   from them, not from `@sub` models.
2. **A blob shadow** (`taguchi/Other/shadow.tm2`). It is only used when the weather byte is 2 or 3.
3. **The ball's own `ballshadow.mdl`**, which the port already drew.

Only courts 04 (`net_shadow@sub`) and 09 (`netshadow@sub`) have `@sub` shadow models. P17a already draws those.

## Projected shadow system (engine object at 0x1ea5d8)

- **Receivers.** The stage setup (0x32d4e0) registers one receiver: the hole's ground model, at stage +0x138,
  through 0x16b180 and 0x14d3d0.
- **Casters.** Casters are registered with 0x16b2b0(sys, obj, dynamic).
  - **Static casters.** These are plant records in categories 0x11..0x13 (tree, prop, structure) whose code
    byte 3 is not `'0'`, as long as their model is not `$off`. Court 10 has 17 such records, which matches
    the 17 static casters in RAM.
  - **Dynamic casters.** There are 8: the players and, I assume, their rackets.
- **Sun direction** (0x32e190, stored at 0x1e7d90):
  - The formula is m = Rx(−(start + (hour−1)/17·(end−start))) · Rz(tilt) · Ry(−π/2 − azimuth°).
  - The direction is m's third row.
  - start, end and tilt come from `envir_cNN.dat` at +0x10 + k·0x30 (+4, +8, +12). The azimuth comes from
    `envir_cNN_h01.dat` at +0x58.
  - The result matches RAM to 1e-5 for slot 5 (court 10) and slot 3 (court 04). See the unit test in
    `crates/hst/src/shadow.rs`.
  - When the shadows are set up, the elevation is clamped to at least 50° (the global at 0x3fbee8), keeping the
    heading.
- **Strength** (0x32f0d0, CLUT built at 0x16a830):
  - S = ⌊envir +0x410 + k·0x10 × 255⌋. The CLUT entries are (S·i)>>8 in each of R, G, B and A.
  - The top entry is 0x2c on court 10 and 0x4b on court 04, which matches RAM.
  - Weather 1, 2 or 3 scales S by 0.75, 0.5 or 0.25.
- **How the ground is darkened.** The shadow multiplies the ground colour. It does not subtract from it.
  - Measured on `orig_slot5.png` under the umbrella shadow: the ground drops from (78,90,37) to about (48,46,22).
    That is a factor of about 0.6, including the blue channel. A subtraction would have taken blue to 0.
  - 1 − 44/128 = 0.66, so the blend is GS (0 − Cd)·A + Cd in gamma space.

## Port

- `shadow.rs` reads the sun and strength.
- The match's DirectionalLight is aimed along the sun direction.
- Only meshes under a `Rig` or a `Caster` cast shadows. Everything else gets NotShadowCaster: the ball, the effects
  and the NPC capsules.
- GsMaterial now casts through `gs_prepass.wgsl`, which keeps the alpha test so that leaves cast leaf shapes.
  Alpha-tested draws are `AlphaMode::Mask`.
- The hole's draws get `shadow = darken`. gs.wgsl multiplies their colour by 1 − darken·(1 − visibility) before
  converting to linear. This darkens every layer the same way, so the result equals darkening the frame buffer.
- The result is `context/shots/p17/port_c10_shadows.png`. The umbrella and tree patches fall to the right and
  toward the camera, as on the PS2, and the players have shadows at their feet.
- The PS2 shadow edges are blurrier because its shadow texture is low resolution. The port uses Bevy's default
  filtering.

## Not done

- Time of day (k) and hour are fixed at 0 and 1; P17e will pick them.
- Weather scaling and the blob shadow for weather 2 and 3.
- The case where an envir strength of 0 falls back to a light field (0x1e7d84 × 128). No court uses it at k = 0.
