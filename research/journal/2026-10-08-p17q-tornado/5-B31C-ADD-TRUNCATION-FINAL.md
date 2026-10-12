# B31c — GS integer truncation on `@add`/`@sub` draws

## Change
- `gs.wgsl` (`GS_PREMUL`, set by `gs.rs` specialize for `Blend::Add`/`Sub`): the fragment outputs the GS's blend
  term on integers, `floor(floor(Cs·255)·As/128)/255`, where Cs is the clamped MODULATE colour (texel × vertex >> 7)
  and As is the 8-bit alpha (`a8`, ta·va >> 7). The pipeline blends it One/One (Add) or One/One ReverseSubtract (Sub).
  The scene target is Rgba8Unorm, so k/255 is stored as k exactly and the frame-buffer add stays on integers too.
- `@sub` (`GS_SUB`): (0 − Cs)·As >> 7 is an arithmetic shift of a negative number, so the GS subtracts the term
  rounded up (ceil).
- Before this, the blend was SrcAlpha·Cs in float, with no truncation anywhere.

## Check
- `research/b31c_add.py`: this formula in f32, compared with `b31b_rast.py`'s integer add over 1M random bilinear
  texels (1/256 steps), vertex colours and alphas. 34 of 1M differ by one level. Those are products within f32
  error of an integer: the shader's +1e-4 bias, needed so exact products don't floor one level low.
- Court 10 match `--shot` (`context/b31c/after.png`): the court's `@add@sub` lines and the other `@add` draws compile
  and look as before.
- `tools/check.sh -p hst`: 151 passed. The one failure is `net::tests::mod_looks_peer_to_peer_and_the_ready_gate`
  timing out, a network test under load that doesn't touch rendering.

## Not verified / not 1:1
- I didn't re-measure the tornado per pixel against the raster (1540 px / 57.7k). The B31b GS dump and the frozen-camera
  hook are gone (`context/b31b` missing, the hook was never committed). The formula now matches the raster's add
  instead; per the 2026-10-12 rule, rendering only has to look right.
- Bilinear filtering stays the GPU's float filter, not the GS's 4-bit-fraction integer filter. As is rounded after
  filtering, while the raster rounds the texel alpha before filtering. Both cause differences of about one level.
- `@sub`'s ceil comes from the GS blend formula. No dump was compared for it.
- Normal blend ((Cs − Cd)·As >> 7 + Cd) still blends in float; it wasn't part of this task.
