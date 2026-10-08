# P17s4: the ground's vertex colours get an HSL shift at load

- The court-4 "ground" in `context/p17s2/s4.gs` is the hole model `GRD01.XB/.../hole/park_h01_s1111.mdl`. Its
  colours in RAM differ from the file. With RAM's vertex colours it fits the court light block exactly
  (e.g. 113·1.1725 → 132).
- At load, the court loader shifts the hole model's (manager +0x6c) vertex colours in HSL:
  - hue = (envir_cNN.dat +0x4e8 + envir_cNN_h01.dat +0x60) % 360 (C remainder, i32; row index is always 0);
  - sat = clamp(+0x4ec + h01 +0x64, −100, 100);
  - light = clamp(+0x4f0 + h01 +0x68, −100, 100).
  - It skips the shift when all three are 0. The original colours are backed up once.
- Per colour, when the shift isn't (0, 0, 0):
  - rgb→hsl on byte/255 (div). With mx == mn, s = h = 0. Otherwise s = d/(mx+mn) when l ≤ 0.5, else d/((2−mx)−mn).
    The hue terms are dr = (0.16666667·(mx−r))/d etc.; h is db−dg, (dr+⅓)−db or (dg+⅔)−dr; then wrap h to [0, 1].
  - h += hue/360, wrapped with while-loops; s and l get + sat/100 and + light/100, clamped to [0, 1].
  - hsl→rgb: q = l·(s+1) when l ≤ 0.5, else msub(l+s, l, s); p = 2l − q. For t = h+⅓, h, h−⅓ (each wrapped):
    p + t(q−p)/0.16666667 when t < 1/6, q when t < ½, p + (⅔−t)(q−p)/0.16666667 when t < ⅔, else p.
    Each channel is truncated after ×255; alpha is kept.
- Court 4: envir row 0 is (0, 0, 0) and h01 is (0, 0, 8), so the shift is lightness +8 %. Examples:
  grey 39 → 59, (0, 2, 0) → (0, 42, 0), (70, 60, 49) → (93, 80, 65), (94, 98, 110) → (112, 117, 131).
- A float32 (round-to-nearest) Python model matches 4702 of 4710 vertices; the 8 misses are off by 1. With
  `hst_sim::ps2` operations (chop rounding; div rounds to nearest), the Rust port matches all 4710 (805 distinct
  colours) against slot 4's RAM.
- Ported: `gs::hole_hsl` (the parameters) and `gs::hsl_shift` (one colour), tested by gs.rs `hole_hsl_shift`.
  main.rs's `shift_colours` applies them to the `hole`-dir model's mesh colours before it is spawned.
- Not explained yet (gap P17s5):
  - The net (`znet`, `netmoto`) wire and pole draw at a constant ⌊vc·1.8⌋ for every normal. 1.8 is the models
    block's A + L, as if d = 1.
  - `house` and `light4` fit neither the court block nor the models block, even with a fitted rotation and normal
    scale.
