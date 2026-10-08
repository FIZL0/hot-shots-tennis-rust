# P17v5 — caster light matrices in the game's floats

All 17 court 10 light matrices are now built from the disc. Each equals the game's (`context/p17v/rcv.txt` `L` rows,
slot 5) bit for bit. Test: `hst-sim/tests/shade.rs` `light_matrices_match_the_game`.

## Inputs (all checked bit-exact against slot 5 RAM on the way)

- **Item matrix:** `world::place` from the plant record. When the plant's scale ≠ 1, the 3×3 is then multiplied
  element-wise by the scale (`vu0::mul`).
- **Box:** the caster model's root node header: lo at +0, hi at +0x10 (`mdl::Model::lo`/`hi`). The centre is
  (lo+hi)·0.5 and the half-extents are (hi−lo)·0.5. The box is **not** grown ×1.1. Journal 10 of the
  2026-10-07 P17 entry got that wrong: the ×1.1 is a different slot, which the light matrix doesn't use.
- **Sun** (`shade::sun`):
  - Inputs:
    - `envir_c10.dat` row 0: start, end and tilt at 0x14/0x18/0x1c.
    - The hole's `envir_c10_h01.dat`: azimuth at 0x58.
    - Hour 1.
  - pitch = −lerp(start, end, (hour−1)/17).
  - yaw = −π/2 − deg·azimuth, wrapped by ±2π.
  - The light is row 2 of Rx(pitch)·Rz(tilt)·Ry(yaw), normalized. It equals RAM's light vector.
  - The shade sun is rebuilt from that light's heading and elevation (`libm` atan2f/sinf/cosf). The elevation is
    clamped to ≥ 50°.
- **Axes** (`shade::sun_axes`):
  - lu = (0,0,1) × sun, normalized (or (1,0,0) when |z| ≥ 0.99999).
  - lv = sun × lu.

## Build (`shade::light_matrix`)

1. W = item × 1/scale. The centre is taken through W and divided by its w.
2. Scale s = 1 / max(Σ|aᵢ·lu|, Σ|aᵢ·lv|), where aᵢ = Wᵢ·halfᵢ.
3. Rows [lu; lv; sun; s·origin], all × 1/o.w.
4. Generic 4×4 inverse (cofactors, det, ×1/det).

The inverse is ~140 FPU ops whose exact order matters. I didn't hand-port it. A scratch symbolic executor walked the
function's disassembly (taken from RAM, since the overlay isn't in the ELF) and printed each FPU op as a
`ps2::` let; dead lets were then pruned.

**Lesson:** `adda.s 0, x` must stay `ps2::add(0.0, x)`, never x. With x = −0 it gives +0. Folding it away left
L[2][3] as −0 on 10 casters.

## Gaps

- **P17v8:** `shade::build` and the app still use the f64 light frame and planes and the app's Rust-trig sun.
  Switch them to `shade::sun` / `light_matrix`.
