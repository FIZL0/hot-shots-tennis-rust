# P17k — weather, exact port (P17i's gaps)

Checked against slot 5 (court 10, bot-only doubles) under PINE. Captures are in `context/p17k/`.

## Schedule (`hst_sim::weather::{Rand, Mt, schedule}`): bit-exact

- The game draws the schedule from its shared MT19937 (the match object's generator). It is seeded with one
  newlib `rand()` output when the match is set up.
- `rand()` is the 64-bit LCG (×0x5851f42d4c957f2d + 1). Its state is 1 at boot and the game never reseeds it. Each
  call returns bits 32..62.
- `research/p17k_schedule_rec.py` records the `rand()` state, the match seed and the schedule.
  `research/p17k_seed_search.py` walks the LCG back and tries each earlier output as the MT seed. Slot 5's seed is
  0x28c7c4a1, and its schedule comes out bit-exact (`hst-sim/tests/weather.rs`).
- `HST_WEATHER_SEED=<hex>` gives the seed directly. Otherwise a clock-picked number of `rand()` steps stands in
  for the menus' calls: those depend on the player's menu input, so there is no single count to port.

## Rain ripples and voices (`play/weather/rain.rs`)

- Each 3 m cell is 2 × 2 tiles, and each tile shows the next of the texture's four frames. The right-hand tiles are
  mirrored in U and the far tiles in V, so the four tiles meet edge to edge.
- The four rain voices (bearings 90/135/225/270) each sweep ±45° about their bearing, one degree every 5 ticks.

## Wind-rate deformer: costume `.NOI` (`hst_sim::noise`, `crate::noise`)

- The deformer is not tree sway. It is the costume noise deformer: a body's `<body>.noi` sways the hair, skirts
  and ties of the nodes it names. NOI layout:
  - the count at +0;
  - 0x30-byte entries from +0x10, each with the node (u16 at +2), the period (+8), the rate (+0xc), the amp
    (+0x10..0x18) and the name (+0x20).
- EE side, bit-exact against PINE (`research/p17k_noise_rec.py`, `context/p17k/noise5.txt`):
  - wind = madd(0 + 0.2, 0x3db60b61, speed), set at every game. It is 1.0 before the first game.
  - freq = 500 · (1/period).
  - Per frame (dt 1):
    - amp = wind · (0x3b173ca7 · amp.x);
    - rate = wind · (0.1 · rate);
    - prev = phase;
    - phase += dt · rate, wrapped into [0, 32).
  - A motion set calls reset(0), which sets the phase to 0, steps once and sets prev = phase. The frame's own
    step follows.
  - Slot 5 at speed 2: wind 0x3ec16c17, first phase 0x3d1abcdf, amp 0x3ae4892a.
- VU1 side, ported from the micro program (ELF 0xc2e84, entries 0x78d/0x78e), decoded with
  `research/tools/vu0dis.py`. MTIR/RINIT/RXOR now print the fsf lane, which the decode needed.
  - The table: RINIT with the game's π (0x40490fd0) gives R = 0x3fc90fd0. Each RNEXT (an LFSR on bits 4 and 22)
    gives r, and entry k = (r + 2) · ±0.25 (+ for even k).
  - Per position entry:
    - s = (x+z, y+x, z+y);
    - t = prev + s · freq;
    - floor and frac by truncation (a whole t gives frac 1);
    - lane x reads z's noise, y reads x's and z reads y's;
    - n = a + frac · (b − a);
    - p += amp · n · weight.
  - Weights are per position entry, from the packet's extra VIF block (S-32 unpack): `mdl::Packet::noise`. The
    owning node is at batch header +0x2a (`Packet::group`).
- Not capture-checked: PINE can't read VU1 memory, so the VU half is checked only against the decode.
- Gaps (P17m):
  - The game moves the entries in bone space before skinning. The port rotates each offset into the bind pose and
    adds it to the drawn vertex.
  - A restart of the same motion under the same serial doesn't reset the deformers.

## Character lighting mode in rain: matches (gap P17l for sunny weather)

- A flag, set at every game, is weather < 2. A second flag, with the same condition, gates the sun flare (already
  ported).
- In clear or cloudy weather, each player's and NPC's model light scale comes from a lookup into the court's
  sun-shade map, a 0x500 × 0x380 bit map built at load.
  - The court's shadow casters are drawn top-down at 1280×896, and a pixel > 0x6f marks shade.
  - The court object keeps the map's origin and scale. On court 10 (slot 5, PINE): origin (−141.486, −117.808),
    scale 4.272 / 3.987.
- In rain, and on court 7, the scale is forced to 1.0, the model default. The port has no shade map, so it draws
  1.0 in every weather: right in rain, but the shade in sunny weather is missing (P17l).

## Particles in the fogged pass

- The rain streaks, ground ripples and leaves now draw as unlit `GsMaterial`s: textured, modulate, normal blend,
  with GS fog. `weather::apply` gives them the court's main fog along with every other GS material.
- `gs.wgsl` skips the VU1 lighting for meshes without normals (`VERTEX_NORMALS`). All court and character meshes
  have normals, so nothing else changes.
- Their textures switch to raw `Rgba8Unorm`, as the GS works on stored values.
- Checked with `HST_WEATHER=3 HST_AUTOPLAY=1 hst <iso> --play --stage 10 --shot`: the streaks and ripples draw as
  before.
- Gap (P17n): no Z write and no alpha test, which carries over from the StandardMaterial draws. The game's
  TEST/ZBUF for these draws hasn't been read.
