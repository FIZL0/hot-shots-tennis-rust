#!/usr/bin/env python3
"""B360: does the original's court look change at a change of ends when the weather schedule changes?
Usage: b360_weather_shots.py <slot> <outdir> <weather> [after_frames…]. Loads the slot (a bot match), writes
<weather> into every entry of the match's weather schedule (gm+0xd8, 65 bytes) and runs on: screenshots at load,
just before the change of ends (phase gm+0x55 == 1) and the given numbers of frames after it ends.
The schedule is only read when a point is set up, so the shot before the change of ends shows whether the game
re-applies it each point. Run under tools/pcsx2.sh."""
import os, subprocess, sys, time
T = os.path.join(os.path.dirname(__file__), "..", "tools")
sys.path.insert(0, T)
from pine import Pine
VSYNC, GM_PTR = 0x1d5780, 0x422f80
p, slot, out, w = Pine(), int(sys.argv[1]), sys.argv[2], int(sys.argv[3])
afters = [int(a) for a in sys.argv[4:]] or [200, 600]
os.makedirs(out, exist_ok=True)
log = open(os.path.join(out, "log.txt"), "a")
def phase():
    return p.read8(p.read32(GM_PTR) + 0x55)
def run(until, cap=60000):
    last = p.read32(VSYNC)
    for _ in range(cap):
        last = p.next_frame(last)
        if until(last):
            return last
    sys.exit("never reached")
def shot(name):
    subprocess.run([os.path.join(T, "screenshot.sh"), os.path.join(out, name)], check=False, stdout=subprocess.DEVNULL)
    log.write(f"{name} vsync {p.read32(VSYNC)} phase {phase()}\n"); log.flush()
p.load_state(slot); time.sleep(0.5)
v0 = p.read32(VSYNC)
run(lambda v: v >= v0 + 30)
shot("0-load.png")
gm = p.read32(GM_PTR)
log.write(f"schedule before {p.read_block(gm + 0xd8, 65).hex()}\n")
for a in range(gm + 0xd8, gm + 0xd8 + 64, 4):
    p.write32(a, w * 0x01010101)
p.write32(gm + 0xd8 + 64, (p.read32(gm + 0xd8 + 64) & ~0xff) | w)
run(lambda v: phase() == 1)
shot("1-ends.png")
run(lambda v: phase() != 1)
end = p.read32(VSYNC)
for a in afters:
    run(lambda v: v >= end + a)
    shot(f"2-after{a}.png")
print("done")
