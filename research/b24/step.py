#!/usr/bin/env python3
"""B24 prototype: lock-step capture. Load a slot, pause, then per frame: FrameAdvance (pad R3) → wait for the
vsync counter to move → read while paused. Usage: step.py <slot> <frames> <out.bin> (record_live.py's layout)."""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "../../tools"))
from pine import Pine, VSYNC
from vpad import send
GM_PTR = 0x422f80
p, want, out = Pine(), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(int(sys.argv[1])); time.sleep(0.3)
gm = p.read32(GM_PTR)
r = [(p.read32(gm + 0x88), 0x290), (p.read32(gm + 0x98), 0x290), (0x3165f0, 0x40)]
send(["press guide 50"])
while p.status() != "paused": time.sleep(0.005)
last, n, t, waits = p.read32(VSYNC), 0, time.monotonic(), 0
while n < want:
    send(["press r3 4"])
    while (v := p.read32(VSYNC)) == last: waits += 1
    if p.status() != "paused":
        while p.status() != "paused": pass
        v = p.read32(VSYNC)
    if v != last + 1: print(f"stepped {last}->{v}", flush=True)
    last = v
    out.write(struct.pack("<I", v) + p.read_regions(r)); n += 1
print(f"done {n} in {time.monotonic() - t:.1f}s, waits {waits}")
send(["press guide 50"])
