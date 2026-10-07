# P0c2 — collision world data (done)

Evidence: `crates/hst-data/tests/collision.rs` vs `context/ram/s05.bin` (court 10): 15 models, 6368 triangles,
node matrices, two-sided flags and attribute maps bit-exact.

## Objects the live ball queries (0x32f690 args from 0x375e30: t3=1, stack byte 0)
- court object = `***(world+0x138)`, world = `*(gm+0x84)`, gm = `*0x422f80`. Model instance = obj+0xc,
  obj+0x120 scale. Court 10 = GRD01.XB `greece_h01_s1111.mdl`.
- world grid (gate `0x3fc008` = 1 in s03/s04/s05): cells 20 m, bounds ints at 0x4238e0 (+8 stride: xlo,xhi,ylo,yhi,zlo,zhi),
  dims 0x423918/0x423920, cell = {i32 n, ptr list} at `*0x4238a8 + (d2*(y + x*d1) + z)*8`; object = `***entry`.
  65 objects, 14 models (HOL01.XB): znet_s1000 (the NET, plant record cat 17 idx 11 at origin), shinpan (umpire chair),
  zseat, stockage, gate, debrisc, trees/herbs/flowers. Sphere pre-test radius = obj scale * `*(*(node0 def)+0x30)` + sweep r.
- world+0x14c object (0x1909160) is NOT queried by the live ball (stack byte 0).
- Model instance +0x10..+0x4c = world→model matrix (inverse of plant pos/yaw/scale: stockage pos (-7.881,.,22.416) →
  row3 (7.881,-0.023,-22.416); shinpan yaw π, scale 1.022 → (6.589,0,-0.003)). Sweep radius /= scale.

## Model data (`*(inst+0x54)`) — the .mdl is loaded raw at `*(data+0)`, size `*(data+4)`
- data+0x1c → {?, n, mats[]}; mat → {?, n batches, batches[]}; batch struct +4 header (file), +0x14 packets[].
- packet struct: +4 header (file 0x60), +0xc VIF data, +0x10 slot→vertex map ("bone list"), +0x14 tri starts (+0x38 bytes),
  +0x18 bounds (32 B each), +0x24 constant colour (header +0x4c < 0).
- header +0x44 pos, +0x48 normals (V4-16; +6 low byte >>3 = flags; +7 bit7 = restart, skipped), +0x4c colours, +0x50 UVs —
  qword offsets into VIF data. Positions/normals indexed via map[s+k], UVs/colours by slot s+k directly. UV.w = ±1 winding.
- data+0x30 → per-node table (12 B: ?, n, (material,batch,...) 4-byte entries), built at load by 0x14ad30-ish (decomp line
  ~53190): batch included iff material has an attribute map (+0x94) and batch header +0 == 1; order material asc, batch asc.
- data+0x40 → material records 0x60 stride: +0x10 → state, `+0x10` flags = (+0x18≠0)<<4 | (0x14≤hdr1e≤0x1d)<<2 |
  (10≤hdr1e≤0x13)<<1 | (hdr+0x24≠0) | 8 when textured (decomp line ~47585, 47809). Flag 1 = two sided (2 passes).
  +0x58 → +0x94 attribute map = MTL embedded texture `hdr+0x22`; map +4 header, +0xc texels, +0x28 palette, +0x2c table
  (MTL's 16-byte map after the palette; RAM sets unused palette entries to 0xff).
- node instance (inst+0x64, 0x120 stride, tree via +0x104 children / def +0x1c count): +0x108 def, +0x10c pre-order
  index; def+0x10 → file 3×mat4, +0x80 = model→node matrix (identity for all court-10 models).
- Loading patches bytes outside collision data (batch header +0x2c etc.): 187 bytes in the court model.

## For P0c3 (hit material)
1580b0: no map → material 4; else texel of UV (wrap byte at `*(rec+0x10)+0x20`, 0x0c on court 10) → table → id;
0 → hit+0x44 default. Materials seen live: 1 court, 13 outside, 48 fence, 26 net body, 2 net cord.
