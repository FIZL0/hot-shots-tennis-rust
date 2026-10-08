# P17l — sun-shade map

Checked against slot 5 (court 10, bot-only doubles) and slots 3/4 (court 4) under PINE. Captures are in
`context/p17l/` (RAM dumps, the game's maps, the port's maps, the ball recording, `cmp.py`).

## Who is shaded

- Six update routines look the map up and set a model's light scale. The ball is one of them, and so are several
  NPC kinds (some only on one court, some gated by a court-object flag). No player routine reads the map. In
  slot 5 two players stood on shaded pixels and none of the players' model scales moved off 1.0; only the
  ball's did.
- Ball: under a unit above the ground the scale is max(height, lookup). Below the ground it is at least the
  smallest denormal (`hst_sim::shade::ball`). The decompiled clamp reads as a min with a denormal; the recording
  shows the ball resting in full shade with its scale equal to its height (0x3d83ba34 ≈ 0.0643), so the compare is
  the other way round. On the court the ground is y 0. Off it (|x| > 10.685 or |z| > 19.885) the game ray-casts
  the ground model, and when the ray misses it leaves the last scale.
- NPCs: the lookup at their position, always.

## Map frame (`Frame::new`): bit-exact

- The box is the union of the hole model's packet header boxes (+0 min, +0x10 max). That is not the node or
  model box.
- The camera height is worked in double precision with the C library `tan` of half the 40° field of view. Every
  double → float conversion truncates (`ps2::chop`). Round-to-nearest leaves a 1-ulp mismatch.
- Matched on court 10 and court 4 (`frame_matches_game_ram`).

## Lookup (`lookup`): bit-exact

- row = cvt(scale_x · (x − origin_x)), column = cvt(scale_z · (z − origin_z)), read unsigned. The result is 1.0
  unless row < 0x37f and column < 0x4ff. The four bits are blended bilinearly (a set bit means 0, a clear bit
  means 1), with PS2 FPU rounding.
- 1299 lock-step frames of the slot 5 rally were checked through `shade::ball` with the game's own map. 1268 are
  bit-exact, including partial bilinear values as the ball rolled into a shadow. The other 31 are all off the
  court: either the ray-cast ground height differs by a few ulps, or a ray miss left a stale value.
- With round-to-nearest maths instead of `ps2` ops, those partial values miss by an ulp.

## Map build (`rasterize`): approximate (gap P17o)

- The port projects the casters' triangles along the sun direction onto the hole's top surface, as a binary
  silhouette.
- The game's 17 static casters are the port's `Caster` plants, with the same models, positions, yaws and
  scales.
- The maps still differ. IoU is 0.23 on court 10 and 0.24 on court 4. The port's shadows are larger and partly
  offset: the colonnade footprints sit about 14 units off in z and 5 in x, and the port has one colonnade shadow
  the game lacks.
- The game renders the shadow textures through the GS and reads the red channel back (> 0x6f), so the size,
  filtering and projection of those textures decide the footprint. Porting that is P17o.

## Light scale in the port

- The game multiplies only the VU1 directional light colour by the scale. Port models are unlit, so `shade.rs`
  darkens half the colour (`DIRECT`) until models are VU1-lit (P17p).
- The ball is tagged `shade::Ball`. NPC rigs get their own materials. Players are skipped.
