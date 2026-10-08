# P17q done: latch, in-between, look check

## Ported
- Latch (`play/tornado.rs`): a player's `contact` / `serving.swing` turning Some latches; the contact frame's
  `hit_effect` starts the tornado only when latched; the latch clears outside Serve/Rally.
- Slow-mo in-between: `hst_sim::tornado::Tornado::between` (scale, uv time, matrix, alpha with the exe floor
  `Game::tornado_floor`). Not wired: the port has no slow motion; whoever ports it (P0b4d) calls `between` on
  in-between frames with the slow-mo fraction and the UVA clock's last/this time.
- Flash quad: dead in retail (1-FINDINGS), documented in the sim module, not drawn.

## Tests
- `tornado_slowmo` (hst-sim/tests/tornado.rs): 114 in-between frames bit for bit (scale, uv time, key, alpha)
  against `context/fixtures/tornado_slowmo.bin`, recorded by `research/p17q_slowmo_rec.py` on slot 5 with
  forced ×4 slow-mo (pokes the slow-mo block whenever the tornado is on). Layout in the test.
- `tornado_s05` unchanged (22 starts, 583 frames on).

## Look check
- `research/p17q_freeze.py 5 1.0` freezes the original (slow-mo one step per 3000 frames) on a full-grown
  tornado; screenshots with the tornado on, then its on byte (+0x62) cleared → `context/p17q_orig_on/off.png`,
  the difference is the tornado alone. Port: same on/off pair from a temporary freeze hook (not committed:
  pause `Time<Virtual>` once `t >= end`, screenshot, hide, screenshot).
- No frame-matched pair is possible (the port can't replay the original's rally), but this pair is close:
  same court, same shot direction, scale 5.33 (orig) vs 6.09 (port), cone ~90 vs ~80 px long at the same
  court size on screen.
- Materials: all five `@vert@add` (Cs·As + Cd, MODULATE), MTL alpha 0x40 (b: 0x80), wrap repeat, no mips,
  textures: aura PSMT4 grey (mostly ≤ 0x6a) and a white-RGB alpha-streak texture. The port's setup matched,
  except the texels were sRGB-decoded: now raw (Rgba8Unorm), as gs.rs.
- Summed added brightness (8-bit, pixels with delta > 3): orig 94.6k over 3670 px, port 106k over 2433 px;
  port percentiles 50/90/99 = 27/111/162 vs 16/62/110. So the port is about as bright in total but packs it into
  fewer pixels, partly the view (cone 0.9× as long), partly linear-light blending: adding x in linear brightens
  dark channels more than the GS's gamma-value add (grass blue 0x33 + 0x4c: GS +76, port +105), so it reads
  whiter. That is every 3D translucent/additive draw (court `@add`, effects), not the tornado's: new task B31.
