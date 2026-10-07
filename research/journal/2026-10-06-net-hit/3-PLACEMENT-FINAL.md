# P0c2b — collision world placement (done)

Evidence: `crates/hst-sim/tests/world.rs` vs `context/ram/s05.bin` (court 10): 117 props in list order, model,
net-post flag, scale, world matrix (obj+0xe0), root node matrices (node+0x00/+0x40), sphere centre (node+0xc0),
collision flag (data+0x48), world→model (inst+0x10) all bit-exact; grid box (0x4238c0/d0), cell bounds, dims and
all 704 cell lists (525 entries, 66 colliding props) exact. Asm: `context/notes/asm_place.txt`, `asm_grid.txt`.

## Which records (0x32d4e0, per category 0..0x17, records in file order; 0x332bf0 → 0x333370 builds 0x90 records)
- categories 0xf, 0x11–0x14 with a model in the category table and scale (file +0x24) ≠ 0 → 0x335eb0.
- 0x90 record: +0x10 identity 3×3, +0x40 pos (file +4) w=1, +0x50 file+0x10, +0x54 yaw (file+0x20), +0x58 scale,
  +0x68 code (file+0x28).
- list 0x423928: 0x336520 links each new object right after the head → order [first, last, …, second].
  0x423930 = count (117). Grid takes those whose model data+0x48 ≠ 0 (set at load iff some node has a collision batch).

## World matrix
- M = I with row3 = pos; M = rotY(yaw)·M (0x125f68 on identity, then 0x125a80 mat×mat = rows of 2nd arg through 1st).
- cats 0x11–0x13: code byte1 ≠ 0 → tilt rotX(d·π/36, > π/2 → −π) (0x125ec0); byte0 ≠ 0 → rotY(d·2π/36 folded to ±π).
  digit = signed char < ':' ? c−'0' : c−'7'.
- cat 0x11 and sqrt(z·z (mula) + x·x (madd)) < 1 → identity, object byte +7 = 1 (the net, znet at origin).
- sin/cos 0x125da0: x = π/2 ∓ θ (FPU), odd poly x + c3x³ + c2x⁵ + c1x⁷ + c0x⁹ on VU0 (consts 0x362e9c14, 0xb94fb21f,
  0x3c08873e, 0xbe2aaaa4) = cos θ; sin = ±sqrt(1 − cos²) via Q.
## Instance (0x14c440 → 0x14ca00)
- obj+0xe0 = M. scale ≠ 1: S = rows 0..2 xyz × s (vmulx.xyz); inst+0x10 = inverse(M) (0x125b98: transpose, −t·Rᵀ, keeps
  t.w) × diag(1/s div.s); else S = M, inverse only. Node 0x14cb50: node[0..3] = local·S, [4..7] = local·M,
  +0xc0 = node[0..3] applied to header centre. local = node file matrix #2 (def+0x10 +0x40).
- Global scale DAT_003fbef0 / DAT_003fc020 are 1.0 in s05 (no effect).
## Grid (0x337eb0)
- r = obj scale × header radius (mul.s); box c∓r (sub.s/add.s); global min/max; cell = (v<0 ? v−20 : v)/20 trunc.
- each colliding object appended (list order) to every cell of its clamped box. Cell index (nz·(y + x·ny) + z).

## Fix found on the way
`vu0::add` lost the round-toward-zero step when the addend is below f64 precision of the sum (x + c1·x⁷ for
x ≈ 1.6e-3): now TwoSum-exact. All flight/live tests still bit-exact.
