#!/usr/bin/env python3
"""B174: the original's change-ends glide as shown. Usage: b174_swing_probe.py <slot> <out.txt> [max_frames].
Per change-ends tick: the camera handler's counter, the shown view the blend writes (eye, two rows, fov in degrees),
the reference frame it orbits in (the 4x4 behind the camera handler's +0x64) and the saved start view (+0x100..0x13c), and the pivot: court locator 0x76's position (the court table behind +0xb8)."""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
VSYNC, GM_PTR, SHOWN = 0x1d5780, 0x422f80, 0x1e7d20
p, out = Pine(), open(sys.argv[2], "w")
cap = int(sys.argv[3]) if len(sys.argv) > 3 else 60000
p.load_state(int(sys.argv[1])); time.sleep(0.5)
fl = lambda b: " ".join(f"{x:.4f}" for x in struct.unpack(f"<{len(b) // 4}f", b))
last, n, seen = p.read32(VSYNC), 0, 0
while n < cap:
    v = p.next_frame(last); last = v; n += 1
    gm = p.read32(GM_PTR)
    if p.read8(gm + 0x55) != 1:
        if seen: break
        continue
    seen += 1
    ch = p.read32(gm + 0xbc)
    out.write(f"{v} e8 {p.read32(ch + 0xe8)} shown {fl(p.read_block(SHOWN, 0x34))} frame {fl(p.read_block(p.read32(ch + 0x64), 0x40))} "
              f"start {fl(p.read_block(ch + 0x100, 0x40))} fov0 {fl(p.read_block(ch + 0x140, 4))} pivot {fl(p.read_block(p.read32(gm + 0xb8) + 0xd130 + 0x76 * 0x40 + 0x30, 12))}\n")
    out.flush()
print("done", n, "frames,", seen, "change-ends ticks")
