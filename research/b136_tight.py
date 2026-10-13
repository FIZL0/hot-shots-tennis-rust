#!/usr/bin/env python3
"""B136: tighten rough label boxes to their ink, printed as a `menu_art.rs` table.

    research/b136_tight.py SHEET.png 'x y w h Text' ...

Ink = opaque pixels that differ (> 40 in any channel) from the box's surroundings: the mode of its ring when the
ring is mostly opaque (a label on a pill or bar), else transparency.
"""
import sys
from collections import Counter
import numpy as np
from PIL import Image

a = np.asarray(Image.open(sys.argv[1]).convert('RGBA')).astype(int)
H, W, _ = a.shape
for spec in sys.argv[2:]:
    x, y, w, h, text = spec.split(' ', 4)
    x, y, w, h = int(x), int(y), int(w), int(h)
    ring = [tuple(a[v, u]) for u in range(x, x + w) for v in (y, y + h - 1)] + [tuple(a[v, u]) for v in range(y, y + h) for u in (x, x + w - 1)]
    bg = Counter(ring).most_common(1)[0][0]
    c = a[y:y + h, x:x + w]
    ink = c[..., 3] > 0x20 if bg[3] < 128 else (np.abs(c - np.array(bg)).max(axis=2) > 40)
    ys, xs = np.where(ink)
    print(f'    ({x + xs.min()}, {y + ys.min()}, {xs.max() - xs.min() + 1}, {ys.max() - ys.min() + 1}, "{text}"),')
