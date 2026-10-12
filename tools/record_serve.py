#!/usr/bin/env python3
"""Record the server's serve, every vsync, from a save-state load while tools/vpad.py drives P1 (B29).
Usage: record_serve.py <slot> <frames> <out.bin> [vpad command ...]. Loads the slot, starts sending the vpad
commands (wall-clock, so the presses land at whatever frames they land: the decoded pad is recorded with them),
and writes one sample per vsync: u32 vsync, the decoded pad of port 0 (0x18), globals 0x423040..0x423080 (0x40),
the rally block 0x3165f0 (0x50), the server's (`0x42304c`) player block +0x3d40..+0x4000, its motion
(*(+0x54) +0x20..+0x40), the live ball (*(gm+0x88) +0x60..+0x140) the server's path's word +0x224 (bounces) padded to 8, and the match object's gm+0x50..+0x60
(phase gm+0x55, tick gm+0x58).
Samples where the frame ticks mid-read are skipped (logged)."""
import subprocess, struct, sys, time
from pine import Pine

VSYNC, GM_PTR = 0x1d5780, 0x422f80
p = Pine()
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(slot)
time.sleep(0.3)
if len(sys.argv) > 4:
    subprocess.run([sys.executable, "tools/vpad.py", "send", *sys.argv[4:]], check=True)
last, n = p.read32(VSYNC), 0
while n < want:
    v = p.next_frame(last)
    if v != last + 1: print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    gm = p.read32(GM_PTR)
    pl = p.read32(gm + 0xa8 + 4 * p.read32(0x42304c))
    path = p.read32(pl + 0x13fc) + 0x220
    r = [(0x2efb30, 0x18), (0x423040, 0x40), (0x3165f0, 0x50), (pl + 0x3d40, 0x2c0), (p.read32(pl + 0x54) + 0x20, 0x20),
         (p.read32(gm + 0x88) + 0x60, 0xe0), (path, 8), (gm + 0x50, 0x10)]
    a = p.settle(r, v)
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    out.write(struct.pack("<I", v) + a)
    n += 1
print("done", n)
