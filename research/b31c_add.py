"""B31c: gs.wgsl's `@add` term (GS_PREMUL) in f32 against b31b_rast.py's GS integer add, over random texels (bilinear
fractions included) and 8-bit vertex colours/alphas. Prints the mismatch count (a few per 100k: f32 error at a level boundary)."""
import numpy as np
f = np.float32
rng = np.random.default_rng(31)
n = 1_000_000
T = rng.integers(0, 256 * 256, n).astype(f) / f(256)  # bilinear texel, 1/256 steps
vc = rng.integers(0, 256, n).astype(f)
As = rng.integers(0, 256, n).astype(f)
ref = np.floor(np.floor(np.clip(np.floor(T * vc / 128), 0, 255)) * As / 128)  # rast: Cs = T·vc >> 7, Cs·As >> 7
# shader: in.color = vc/128, t.rgb = T/255, rgb = t·c clamped to 1, then floor(floor(rgb·255 + 1e-4)·a8/128)
rgb = np.minimum((T / f(255)) * (vc / f(128)), f(1))
port = np.floor(np.floor(rgb * f(255) + f(1e-4)) * As / f(128))
bad = (port != ref).sum()
print("mismatches", bad, "of", n)
assert bad < n * 1e-4, bad  # a product within f32 error of the next level (rendering only has to look right)
