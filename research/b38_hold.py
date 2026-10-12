#!/usr/bin/env python3
"""B38: what the screen transition looks like at given counter values. Loads a save slot with a transition running
(9 from b38_transition.py), then for each value holds the global transition object's counter (+0x8) there by
rewriting it while an F8 screenshot is taken. usage: b38_hold.py <slot> <outdir> <value>... Run under tools/pcsx2.sh."""
import sys, time, os, subprocess, threading
sys.path.insert(0, "tools")
from pine import Pine
p = Pine()
slot, out = int(sys.argv[1]), sys.argv[2]; os.makedirs(out, exist_ok=True)
p.load_state(slot); time.sleep(0.5)
t = p.read32(0x1ea5d0)
for val in map(int, sys.argv[3:]):
    done = []
    sh = threading.Thread(target=lambda: (time.sleep(0.5), subprocess.run(["tools/screenshot.sh", f"{out}/t{val:03d}.png"], capture_output=True), done.append(1)))
    sh.start()
    while not done:
        p.write32(t + 8, val); p.write8(t + 0x14, 1) if hasattr(p, "write8") else None
    print(val, "len", p.read32(t + 0xc))
