# P17d: clouds

**The plan's premise was wrong.** Clouds are not category-14 records. Category 14 holds the anchors and paths
that creatures link to (see npc.rs). The `cloud/cloudNN_s1111` models (category 6, like hole/holeend/bg 0..5)
get one automatic record at the origin. A separate cloud system then scatters copies of them.

## What the game does (checked in PCSX2, slot 5, court 10)
- **When.** The clouds are made at court load and remade when the weather changes. They are always made, but
  they move and draw only in **singles**: a court-manager flag is set to `players < 3`. The environment index
  uses the same test: env 1 in singles, env 0 in doubles.
- **How many.** `envir_cNN.dat` (CMN.XB `data/court/plant/`, 0x550 bytes) has an i16 at 0x4d0 + env·2.
  `envir_cNN_h01.dat` (GRD01.XB, 0xd0 bytes) has a u8 percentage at 0x50 + env. The count is short × u8 / 100,
  or 20 when that is 0. Court 10 gives 9 in doubles, which matches RAM, and 11 in singles.
- **Spawn (normal weather).** R = 2000.
  - x and z are uniform in ±R; y is uniform in −200..−400 (game space is Y-down, so these are above the court).
  - The model is picked at random from the court's cloud models. Each is a flat 10×10 quad.
  - Scale 40. The rotation is rotY(random 0..2π) then rotX(π).
  - The clouds are kept sorted by y.
  - Rain weathers (3/5) use 200 clouds, R 50, y −80..−200. They are **not ported**.
- **Per 60 Hz tick.** pos += speed/600·40·(sin θ, 0, cos θ), with θ the wind direction in degrees. x and z
  wrap by ±2R. Measured on court 10 (wind 315°, speed 2): −0.0943, +0.0943 per vsync.
- **Fade.** f = (R² − r²)/(R² − (0.9R)²), clamped to 0..1, with r measured in the xz plane. The RAM value read
  0.39 at r ≈ 1924. The game writes f to the model (+0x5c). How the draw uses it is not confirmed; the port
  scales the material alpha by it.
- **Drawn at** sim position + camera position, so the clouds travel with the eye. The measured offset equalled
  the camera position.
- **Wind** comes from GAME.BIN data, one 0x14-byte row per court (`exe::Game::wind`):
  - i32 chance
  - i32 kind
  - u8[8] allowed directions, indexing [180, 225, 270, 315, 0, 45, 90, 135]°
  - i32 base speed
  The speed is the base when chance > 0, else 0. The direction is picked at random among the allowed ones. Court
  10 allows {315, 0, 45} at speed 2. The per-game gusts in doubles are not ported, since the clouds don't draw
  there anyway.

## Port
- `hst_sim::clouds`: spawn, tick, wrap and fade, with one test against the slot-5 numbers.
- `layout::cloud_count` and `exe::Game::wind`, with a disc test (court 10: wind (315, 0, 45)/2; 9 and 11 clouds).
- `main.rs` (`--stage N --singles`): each cloud gets its own material clone (forced to modulate so alpha scales).
  An Update system ticks the clouds at 60 Hz, places them relative to the camera, and applies the fade.
- The port uses its own xorshift, not the game's MT.
- Check: with a temporary 200 clouds and the camera looking straight up, they draw
  (`context/shots/p17/clouds_200_overhead.png`). With the real 11 spread over 4 km, a given view often shows none.
- Seen while checking: the sky dome has an open top, so the zenith shows the clear colour. That belongs to
  P17e, not this task.
