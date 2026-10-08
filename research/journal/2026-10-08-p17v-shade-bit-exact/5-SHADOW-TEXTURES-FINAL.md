# P17v2: the casters' shadow textures, bit for bit

## Result
- `shade::pass_texture` draws the 16 static casters' 128² PSMT4 shadow textures from the load pass's draws.
  - All 16 equal the pool in `context/p17v/cap0.gs`, texel for texel.
  - The test is `shadow_textures_match_the_game` (crates/hst-sim/tests/shade.rs). It reads `context/p17v/pass.txt`, made by `research/p17v2_fixture.py context/p17v/load10.gs context/p17v/cap0.gs context/p17v/pass.txt`.
- The Python replay (`research/p17v2_replay.py load10.gs pool.npy [384|396]`) is exact at both load frames as well.
- The vertices still come from the GS dump. Building them from the caster models with the light matrix is P17v7, which needs P17v5.

## Capture
- `research/p17v2_load.py` takes a multi-frame GS dump of a match load on court 10 (`context/p17v/load10.gs`).
  - PCSX2's multi-frame dump is a **hold** hotkey: it records while the key is down. Copy 5's ini binds it to F12, and the script sends key-down and key-up with `hyprctl send_key_state`.
  - The 16 static textures are drawn at vsyncs 384 and 396, and blitted to 0x3308 + 0x20·i.
- **Pool C (cap0) is the deterministic PCSX2-SW output of that pass.** A dump taken after the load (after10b) has the same pool.
  - This corrects P17v4's reading of C as a "corrupted" pool. C is what a fresh load draws.
  - A is the pool stored in the save state, which was drawn under other conditions.

## The pass, per texture
1. **Clear.** Four sprites write alpha 0 over the 256² target (FBP 0x8c, FBW 4). FBMSK 0x00ffffff, so only alpha is written.
2. **Caster strips.** XYOFFSET is 0x7800 and the scissor is 4..251. The tests are DATE (destination alpha < 0x80) and ATE GEQUAL 0x60.
   - Solid triangles (prim 4) write the flat vertex alpha 0x80.
   - Cutouts (prim 0x3c, HIGHLIGHT2) write the bilinear texture alpha. They are PSMT4 or PSMT8 through a CLUT, wrap REPEAT or REGION_CLAMP, Q = 1 and Z = 0xffffff.
   - Texture 6 has one MODULATE PSMT8 draw (prim 0x7c, Gouraud, AREF 1).
3. **Halving.** Four bilinear FST sprites halve the target in place (TBP 0x1180, 2 texels per pixel, uv + 0.5). Each texel becomes the 2×2 mean with 4-bit weights.
4. **Copy.** A PSMT4HH → PSMT4 local copy writes alpha >> 4 into the texture.

## Two PCSX2 details that made it exact
- **Texel coordinate rounding** (GSState::FlushPrim). For STQ draws whose Z is constant (or sprites), each vertex's S and T lose their low 9 mantissa bits, plus however many their exponent is below Q's. Before this port, 13 of 16 textures differed.
- **4 lanes, not 8.** Copy 5's PCSX2 runs the 128-bit SW JIT, even though the CPU has AVX2. So fst u/v are `trunc(start) + trunc(d·(lane − skip)) + group·trunc(d·4)`, with skip = left mod 4. With 8 lanes, the per-group truncation errors put the last differing texel (texture 5) on the wrong side of a 1/16 weight boundary.
  - **P17v3:** `research/p17v_gssim.py` uses 8 lanes (`skip = left & 7`). Its span-start u/v residual on tiles 1–3 is very likely the same thing, together with texel rounding on the receivers' STQ.

## Oracle method (for later GS questions)
1. `cut.py` trims the dump just before a texture's halving and pads it with 1200 vsyncs.
2. Play it back with `pcsx2-qt -nogui -- dump` under copy 5's config, with `extrathreads = 0`. Multi-threaded playback grabs came out partial, in 16-row bands.
3. Grab it with F11 via `hyprctl send_shortcut`.
4. Diff the 256² alpha target.

A patched dump with ATE/DATE off shows each pixel's raw last-triangle sample. That is how the few-unit u/v differences were narrowed down to setup and lane stepping.
