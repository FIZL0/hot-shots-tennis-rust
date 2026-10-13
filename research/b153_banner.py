#!/usr/bin/env python3
"""B153: what the players and ball do under the "Start the Match!" banner. Starts a doubles match from save slot 2
(b42_intro_camera.py's route) and logs every frame from the match object's creation until `after` frames past the
first phase-2 frame: vsync, phase gm+0x55, frame gm+0x58, per player motion id/clip (anim *(pl+0x54) +0x20/+0x24),
motion time (+0x3c), position (+0x3d70), state (+0x3fa4, +0x3fa6), and the live ball *(gm+0x88): pos +0xe0,
vel +0x130, flags +0x0..+0x10. Text, one line per frame. Usage (under tools/pcsx2.sh): b153_banner.py <out.txt> [after]"""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad
out = sys.argv[1]; after = int(sys.argv[2]) if len(sys.argv) > 2 else 240
ok = "cross"
p = Pine()
p.load_state(2)
time.sleep(1.5)
vpad("sleep 2500", f"press {ok} 150", "sleep 1200", *[f"press {ok} 150", "sleep 1200"] * 3, "press start 150", "sleep 3500")
t = time.time()
while not p.read32(0x422f80):
    if time.time() - t > 30: sys.exit("no match")
    vpad(f"press {ok} 150", "sleep 2500")
f = open(out, "w"); VS = 0x1d5780; last = p.read32(VS); two = None; n = 0
fl = lambda b, o, k=3: " ".join(f"{x:.3f}" for x in struct.unpack_from(f"<{k}f", b, o))
while n < 3000:
    v = last = p.next_frame(last)
    gm = p.read32(0x422f80)
    if not gm: continue
    ph, fr = p.read8(gm + 0x55), p.read32(gm + 0x58)
    if ph == 2 and two is None: two = n
    if two is not None and n - two > after: break
    row = [f"{v} ph{ph} f{fr}"]
    for i in range(4):
        pl = p.read32(gm + 0xa8 + 4 * i)
        if not pl: continue
        a = p.read32(pl + 0x54)
        an = p.read_block(a, 0x40) if a else bytes(0x40)
        m, c = struct.unpack_from("<ii", an, 0x20)
        st = p.read_block(pl + 0x3fa4, 4)
        pos = p.read_block(pl + 0x3d70, 12)
        row.append(f"| p{i} m{m:#x} c{c} t{struct.unpack_from('<f', an, 0x3c)[0]:.2f} @{fl(pos, 0)} s{st.hex()}")
    b = p.read32(gm + 0x88)
    if b:
        bb = p.read_block(b, 0x140)
        row.append(f"| ball {bb[:0x10].hex()} @{fl(bb, 0xe0)} v{fl(bb, 0x130)}")
    print(" ".join(row), file=f, flush=True)
    n += 1
print(out, "court", p.read32(0x422f90), "frames", n, "phase2 at", two)
