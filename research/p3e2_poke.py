#!/usr/bin/env python3
"""P3e2: who makes the sound generator's draws in one frame. Lock-step from slot 5 to vsync <v>, optionally XOR the
generator's next <n> words with ~0, step one frame, save to slot <out>. Diff the two saves' eeMemory.bin.
Usage: p3e2_poke.py <v> <n> <out-slot> [poke 0|1]. Run under tools/pcsx2.sh with HST_LOCKSTEP=1."""
import sys, os, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine

VSYNC, SND = 0x1d5780, 0x43b1d0
v, n, out, poke = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]) if len(sys.argv) > 4 else 1
p = Pine(step=True)
p.load_state(5)
time.sleep(2)
mt = p.read32(p.read32(SND) + 0x740)
last = p.read32(VSYNC)
while last < v:
    last = p.next_frame(last)
i = p.read32(mt + 0x9c4)
print("at", last, "index", i, flush=True)
assert i + n <= 624
if poke:
    for k in range(i, i + n):
        p.write32(mt + 4 + 4 * k, p.read32(mt + 4 + 4 * k) ^ 0xffffffff)
last = p.next_frame(last)
print("now", last, "index", p.read32(mt + 0x9c4), flush=True)
p.save_state(out)
time.sleep(3)
