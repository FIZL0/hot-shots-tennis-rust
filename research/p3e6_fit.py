"""P3e6: predict each poked cheer mark's quad (the port's formula) and compare with its blob in the screenshot diff.
Usage: python3 research/p3e6_fit.py (reads context/p3e6/)."""
import json, math
import numpy as np
from PIL import Image
from scipy import ndimage

D = 'context/p3e6/'
cam = json.load(open(D + 'cam.json'))
T = [[0.3, 0.1, 0.4, 0.5, 300.0], [0.75, 0.08, 0.4, 0.0, 80.0]]
R = np.array(cam['rows']); V = np.array(cam['view']); P = np.array(cam['proj'])
tan = math.tan(math.radians(cam['fov'] * 0.5))
right, down, ahead, eye = R[0, :3], R[1, :3], R[2, :3], R[3, :3]

def gs(p):
    v = np.append(p, 1.0) @ V
    c = v @ P
    return c[:2] / c[3]

for kind in (0, 1):
    t = T[kind]
    pred = []
    for m in cam['marks']:
        at = np.array(m, float) - ahead * 0.5
        half = (at - eye) @ ahead * tan
        s = t[0] * max(t[1] * half, 1.0) * min(t[2] * half, 1.0)
        at[1] -= t[3]
        z = (at - eye) @ ahead
        if t[4] > 0 and (z <= 0.001 or t[4] <= 240.0 / tan / z):
            pred.append(None); continue
        corners = [gs(at - right * s - down * s), gs(at + right * s)]
        pred.append(corners)
    d = np.array(Image.open(D + f'k{kind}_diff.png'))
    lab, n = ndimage.label(ndimage.binary_closing(d > 0, iterations=6))
    objs = [(sl, (lab[sl] > 0).sum()) for sl in ndimage.find_objects(lab)]
    blobs = [sl for sl, a in objs if a > 800]
    print(f'kind {kind}:')
    for k, q in enumerate(pred):
        if q is None:
            print(' ', k, 'hidden'); continue
        (x0, y0), (x1, y1) = q
        # GS 2048 centre, 640x224 field → screenshot
        W, H = d.shape[1], d.shape[0]
        sx, sy = W / 640, H / 224  # a 224-line field
        bx = ((x0 - 2048 + 320) * sx, (x1 - 2048 + 320) * sx)
        by = ((y0 - 2048 + 112) * sy, (y1 - 2048 + 112) * sy)
        hit = [sl for sl in blobs if bx[0] - 5 <= (sl[1].start + sl[1].stop) / 2 <= bx[1] + 5 and by[0] - 5 <= (sl[0].start + sl[0].stop) / 2 <= by[1] + 5]
        got = [(sl[1].start, sl[0].start, sl[1].stop, sl[0].stop) for sl in hit]
        print(' ', k, 'quad x %.0f–%.0f y %.0f–%.0f' % (bx[0], bx[1], by[0], by[1]), 'blob', got)
