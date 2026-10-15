#!/usr/bin/env python3
"""B377: does the original's weather change at a change of ends step in one frame or fade?
Usage: b377_weather_frames.py <slot> <out.tsv> <weather> [frames_after]. Loads the slot (a bot match), writes
<weather> into every game of the match's weather schedule (gm+0xd8, 65 bytes), and logs every frame from
there until <frames_after> (default 400) frames after the next change of ends (phase gm+0x55 == 1) ends:
vsync, phase, court weather byte (+0x135), the court's look fields (+0x50..+0x64: fog F ×3, z range, light scale)
and the scene light rows the weather setter reads (0x1e7d90, 0x70 bytes). Run under tools/pcsx2.sh."""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
VSYNC, GM_PTR, LIGHT = 0x1d5780, 0x422f80, 0x1e7d90
p, slot, out, w = Pine(), int(sys.argv[1]), sys.argv[2], int(sys.argv[3])
after = int(sys.argv[4]) if len(sys.argv) > 4 else 400
p.load_state(slot); time.sleep(0.5)
last = p.read32(VSYNC)
for _ in range(30):
    last = p.next_frame(last)
gm = p.read32(GM_PTR)
for a in range(gm + 0xd8, gm + 0xd8 + 64, 4):
    p.write32(a, w * 0x01010101)
p.write32(gm + 0xd8 + 64, (p.read32(gm + 0xd8 + 64) & ~0xff) | w)
phase = lambda: p.read8(p.read32(GM_PTR) + 0x55)
f = open(out, "w")
f.write("vsync\tphase\tweather\tlook(+0x50..+0x64)\tlight(0x1e7d10..)\n")
end = None
for _ in range(20000):
    court = p.read32(p.read32(GM_PTR) + 0x84)
    look = struct.unpack("<6f", p.read_block(court + 0x50, 24))
    light = struct.unpack("<28f", p.read_block(LIGHT, 112))
    ph = phase()
    f.write(f"{last}\t{ph}\t{p.read8(court + 0x135)}\t{' '.join(f'{v:.6g}' for v in look)}\t{' '.join(f'{v:.6g}' for v in light)}\n")
    if ph == 1 and end is None:
        end = 0
    if ph != 1 and end == 0:
        end = last
    if end and last >= end + after:
        break
    last = p.next_frame(last)
print("done", out)
