#!/usr/bin/env python3
"""B49: record the original's change-ends phase (gm+0x55 == 1) from a save-state load (slot 5 = bots).
Usage: change_ends_probe.py <slot> <outdir> [max_frames]. Per frame from 5 before to 10 after the phase:
vsync, phase, sub, the shown camera (rows, eye, fov), the 4 players' spots and facing rows; screenshots at
phase ticks 0, 20, 40, 60, 79 (outdir/shot_NN.png). Stops after the first change-ends phase."""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
VSYNC, GM_PTR = 0x1d5780, 0x422f80
p, out = Pine(), sys.argv[2]
cap = int(sys.argv[3]) if len(sys.argv) > 3 else 60000
os.makedirs(out, exist_ok=True)
p.load_state(int(sys.argv[1])); time.sleep(0.5)
log = open(os.path.join(out, "frames.txt"), "w")
f = lambda b, o: struct.unpack_from("<f", b, o)[0]
last, n, hist, tick, after = p.read32(VSYNC), 0, [], -1, -1
while n < cap:
    v = p.next_frame(last); last = v; n += 1
    gm = p.read32(GM_PTR)
    ph, sub = p.read8(gm + 0x55), p.read8(gm + 0x56)
    ch = p.read32(gm + 0xbc); cam = p.read_block(p.read32(ch + 0x5c), 0xa8)
    rows = " ".join(f"{f(cam, 0x60 + 4 * j):.3f}" for j in range(12))
    pl = []
    for i in range(4):
        m = p.read_block(p.read32(gm + 0xa8 + 4 * i) + 0x3d40, 0x40)
        pl.append(f"{f(m, 0x30):.3f},{f(m, 0x38):.3f} fz {f(m, 0x28):.2f}")
    line = f"{v} ph {ph}.{sub} dc {p.read8(ch + 0xdc)} e8 {p.read32(ch + 0xe8)} cam {rows} fov {f(cam, 0xa4):.4f} | " + " | ".join(pl)
    hist = (hist + [line])[-5:]
    if ph == 1:
        if tick < 0:
            log.write("\n".join(hist[:-1]) + "\n")
        tick += 1
        if tick in (0, 20, 40, 60, 79):
            subprocess.Popen(["tools/screenshot.sh", os.path.join(out, f"shot_{tick:02d}.png")])
    if tick >= 0:
        log.write(line + "\n"); log.flush()
        if ph != 1:
            after += 1
            if after == 0 or after == 10:
                subprocess.Popen(["tools/screenshot.sh", os.path.join(out, f"after_{after:02d}.png")])
            if after >= 10: break
print("done", n, "frames, change-ends ticks", tick + 1)
