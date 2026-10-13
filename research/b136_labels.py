#!/usr/bin/env python3
"""B136: find the text labels on a menu sheet (PNG from tm2png) and OCR them, printed as a `menu_art.rs` table.

    research/b136_labels.py SHEET.png [--min-h 8]

Ink = alpha > 0x20; connected components merged into lines (vertical overlap, gap < 0.6·height). Each line is
OCR'd (tesseract, 6× on white) as a first guess: check every string by eye against the sheet before using it.
Output: `(x, y, w, h, "text"),` per line, disc pixels, with the OCR confidence as a comment.
"""
import subprocess, sys, tempfile
import numpy as np
from PIL import Image
from scipy import ndimage

path = sys.argv[1]
min_h = int(sys.argv[sys.argv.index('--min-h') + 1]) if '--min-h' in sys.argv else 8
im = np.asarray(Image.open(path).convert('RGBA')).astype(float)
ink = im[..., 3] > 0x20
lab, n = ndimage.label(ink)
boxes = [(s[1].start, s[0].start, s[1].stop, s[0].stop) for s in ndimage.find_objects(lab)]
merged = True
while merged:
    merged = False
    out = []
    for b in boxes:
        for i, o in enumerate(out):
            h = min(b[3], o[3]) - max(b[1], o[1])
            hh = max(b[3] - b[1], o[3] - o[1])
            gap = max(b[0], o[0]) - min(b[2], o[2])
            if h > 0.5 * min(b[3] - b[1], o[3] - o[1]) and gap < 0.6 * hh and max(b[3] - b[1], o[3] - o[1]) < 1.8 * min(b[3] - b[1], o[3] - o[1]) + 4:
                out[i] = (min(b[0], o[0]), min(b[1], o[1]), max(b[2], o[2]), max(b[3], o[3]))
                merged = True
                break
        else:
            out.append(b)
    boxes = out
boxes.sort(key=lambda b: (b[1] // 8, b[0]))
for x0, y0, x1, y1 in boxes:
    if y1 - y0 < min_h:
        continue
    crop = im[y0:y1, x0:x1]
    a = crop[..., 3:] / 255
    lum = (crop[..., :3].mean(axis=2, keepdims=True) * a + 255 * (1 - a))
    # tesseract wants dark text on white: the bright fill becomes dark
    g = 255 - lum[..., 0] if crop[..., :3][a[..., 0] > 0.5].mean() > 128 else lum[..., 0]
    g = Image.fromarray(np.clip(g, 0, 255).astype(np.uint8)).resize(((x1 - x0) * 6, (y1 - y0) * 6), Image.BICUBIC)
    pad = Image.new('L', (g.width + 60, g.height + 60), 255)
    pad.paste(g, (30, 30))
    with tempfile.NamedTemporaryFile(suffix='.png') as f:
        pad.save(f.name)
        r = subprocess.run(['tesseract', f.name, '-', '--psm', '7', 'tsv'], capture_output=True, text=True).stdout
    words = [l.split('\t') for l in r.splitlines()[1:] if l.count('\t') >= 11 and l.split('\t')[11].strip()]
    text = ' '.join(w[11] for w in words)
    conf = min((float(w[10]) for w in words), default=0)
    print(f'    ({x0}, {y0}, {x1 - x0}, {y1 - y0}, "{text}"), // {conf:.0f}')
