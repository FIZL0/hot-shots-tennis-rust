# P17v3 — the receiver pass, bit for bit with PCSX2 SW

`research/p17v_gssim.py` now equals `context/p17v/cap0.red` on all 4 tiles the dump holds (0 reds differ, was
157 / 14 / 552 on tiles 1–3). The model is ported as `shade::receiver_tile`; test
`hst-sim/tests/shade.rs` `receiver_tiles_match_the_game` (fixture `context/p17v/rcv_pass.txt` from
`research/p17v3_fixture.py context/p17v/cap0.gs context/p17v/cap0.pkl context/p17v/cap0.red context/p17v/rcv_pass.txt`,
run from `research/`).

## What it was

- **Texel coordinate rounding** (`GSState::FlushPrim`, the same rule P17v2 found for the texture pass). It applies
  because the receivers' Z is constant (368) over every draw. Each vertex's S and T lose their low 9 mantissa bits
  plus however far their exponent is below Q's, and Q loses its low 8. That happens before `ConvertVertexBuffer`'s
  S/Q, T/Q, which is where the span-start u/v offset came from.
- **4 lanes, not 8** (copy 5 runs the 128-bit SW JIT, P17v2). With the rounding alone and 8 lanes, 1 + 5 reds still
  differ (tiles 1 and 3). Both are needed.
- Batching was not the cause. The runs of (tile, texture) the sim uses as GS batches decide constant Q correctly:
  128 of 132 batches take the constant-Q path, and 4 divide per pixel. Both paths are exact. The constant-Q test
  is on the raw Q, before the rounding, as in PCSX2 (`m_vt.Update` runs first).
- The sim's constant-Q branch now follows PCSX2 when Q = 1 (fst, no divide). That case doesn't occur in cap0.

## Port (`hst-sim/src/shade.rs`)

- `gs_tri` is `pass_tri`'s setup and edge walk split out. It hands each span to a closure, so `pass_tri` and
  `receiver_tile` share it. `shadow_textures_match_the_game` still passes.
- `texel_round(st, q_exp)` now takes Q's exponent. The texture pass passes 127.
- `receiver_tile(&[ReceiverDraw]) -> 640×224 reds`. Each draw is one batch: a `PassTexture` (128², reds,
  `Clamp(0, 127)`) and its triangles (12.4 XY relative to XYOFFSET, raw S T Q). Each bilinear sample is added and
  clamped at 255.

## Not verified / not 1:1

- **Batch boundaries** are taken as runs of the same (tile, texture). PCSX2's actual flush points are not traced.
  This matters only for the constant-Q decision. It agrees with the reference on all 132 batches, but a different
  build could split a batch elsewhere.
- **Constant Z is assumed** (`receiver_tile` always rounds). It holds for every receiver draw in cap0. A draw with
  varying Z would skip the rounding in PCSX2.
- **The Q = 1 constant-Q branch** is ported from the PCSX2 source but is never exercised by the data.
- **Only 4 of the frame's tiles are in the dump.** cap0.red also has 2 701 non-zero reds at (640, 448), which no
  dumped tile covers. The other tiles are unchecked.
- **The vertices, triangle sets and textures come from the dump.** Building them is P17v1 (vertices, done), P17v6
  (triangle sets), P17v2/P17v7 (textures). Assembling the map from them is the new gap **P17v9**.
- **The reference is PCSX2 SW (128-bit JIT), not a real GS and not the HW renderer** that made map05.bin (see
  journal 1).
