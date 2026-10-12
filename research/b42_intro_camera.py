#!/usr/bin/env python3
"""B42: record the court intro's camera from a menu save state. Starts a doubles match from save slot 2 (pick.py's
route; on pad copy 5 ✕ confirms), optionally picks another court on the confirm screen (Select Court, right ×n),
then samples every frame until the first serve: u32 vsync, gm 0x100, the camera handler *(gm+0xbc) 0x200 and its shot
camera *(handler+0x64) 0x3100 (shot number +0x114, channels +0x2d58 + 0x1c·c, view rows +0x60, fov +0xa4) and the
match phase. Prints the court. Usage (under tools/pcsx2.sh): b42_intro_camera.py <out.bin> [court steps right]"""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad
out = sys.argv[1]; steps = int(sys.argv[2]) if len(sys.argv) > 2 else -1
ok = "cross"
p = Pine()
p.load_state(2)
time.sleep(1.5)
vpad("sleep 2500", f"press {ok} 150", "sleep 1200", *[f"press {ok} 150", "sleep 1200"] * 3, "press start 150", "sleep 3500")
if steps >= 0:
    vpad("press down 150", "sleep 700", f"press {ok} 150", "sleep 2500",
         *[c for _ in range(steps) for c in ("press right 150", "sleep 1000")], f"press {ok} 150", "sleep 2500",
         "press up 150", "sleep 700")
t = time.time()
while not p.read32(0x422f80):
    if time.time() - t > 30: sys.exit("no match")
    vpad(f"press {ok} 150", "sleep 2500")
f = open(out, "wb"); n = 0; VS = 0x1d5780; last = p.read32(VS)
while True:
    v = last = p.next_frame(last)
    gm = p.read32(0x422f80)
    if not gm: continue
    ch = p.read32(gm + 0xbc)
    if not ch: continue
    pl = p.read32(gm + 0xa8)
    if p.read8(gm + 0x55) == 2 and pl and p.read8(pl + 0x3fa4) == 1: break
    cam = p.read32(ch + 0x64)
    if not cam: continue
    f.write(struct.pack('<I', v) + p.read_block(gm, 0x100) + p.read_block(ch, 0x200) + p.read_block(cam, 0x3100))
    n += 1
    if n > 1500: break
print(out, "court", p.read32(0x422f90), "frames", n)
