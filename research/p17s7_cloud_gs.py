"""P17s7: the moving clouds' GS colour (context/p17s7, from research/p17s7_capture.py). Each cloud is one 4-vertex
TME+FGE+ABE strip of vc (128,128,128,128); VU1's unlit path with flag bit 0 gives RGB = ⌊vc·mat·light·(A + L)⌋ from
the `_clo` view's light block (a copy of the main view's) and A = ⌊vc.a·mat.a·fade⌋ (mat.a 0.8 on cloud01, else 1).
  p17s7_cloud_gs.py [dir]"""
import struct, sys, os
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from p17s_light_gs import stream

d = sys.argv[1] if len(sys.argv) > 1 else "context/p17s7"
ram = open(f"{d}/s5.ram", "rb").read()
f = lambda a: np.float32(struct.unpack_from("<f", ram, a & 0x1ffffff)[0])
u = lambda a: struct.unpack_from("<I", ram, a & 0x1ffffff)[0]
view = 0x1e7d10  # main view; the clouds' copy keeps its light block
light = [f(view + 0x60 + 4 * i) for i in range(3)]
k = f(view + 0x70) + f(view + 0x74)
rgb = tuple(int(np.float32(128) * c * k) for c in light)
fades, c = [], u(0x423208)
while c:
    fades.append(f(c + 0x14c)); c = u(c + 0x144)
runs = [r for prim, r in stream(open(f"{d}/s5.gs", "rb").read()) if prim == 0x7c and len(r) == 4 and r[0][3][:3] == rgb]
alphas = sorted(r[0][3][3] for r in runs[-len(fades):])  # the clouds draw last; earlier strips of this colour are other models
print("rgb", rgb, "fades", [round(float(x), 4) for x in fades], "alphas", alphas)
left = list(alphas)
for fd in fades:
    for m in (1.0, 0.8):
        a = int(np.float32(128) * np.float32(m) * fd)
        if a in left:
            left.remove(a); print(f"fade {fd:.4f} mat.a {m}: {a}"); break
    else:
        sys.exit(f"fade {fd} has no strip")
assert not left
print("all", len(fades), "clouds exact")
