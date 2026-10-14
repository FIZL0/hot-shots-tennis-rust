#!/usr/bin/env python3
"""B272: screenshots of the original through the end of its change of ends.
Usage: b272_ends_shots.py <slot> <outdir> <latency> [k…]. Runs the slot to the change of ends once, noting the vsync of
(counter 80, from the counter on the first frame seen in the phase), and saves scratch slot 8 well before it; then for each k reloads 8 and asks tools/screenshot.sh for a shot
`latency` frames before counter k's vsync (the hotkey lands that late; k < 0: −k frames after counter 0). The vsync
count is game RAM, so each reload replays the same frames."""
import os, subprocess, sys, time
T = os.path.join(os.path.dirname(__file__), "..", "tools")
sys.path.insert(0, T)
from pine import Pine
VSYNC, GM_PTR = 0x1d5780, 0x422f80
p, out, slot, lat = Pine(), sys.argv[2], int(sys.argv[1]), int(sys.argv[3])
ks = [int(k) for k in sys.argv[4:]] or [40, 30, 24, 20, 16, 12, 8, 4, 0, -30]
os.makedirs(out, exist_ok=True)
log = open(os.path.join(out, "log.txt"), "a")
def state():
    gm = p.read32(GM_PTR)
    return p.read8(gm + 0x55), p.read32(p.read32(gm + 0xbc) + 0xe8)
def run(until, cap=60000):
    last = p.read32(VSYNC)
    for _ in range(cap):
        last = p.next_frame(last)
        if until(last):
            return last
    sys.exit("never reached")
p.load_state(slot); time.sleep(0.5)
first = run(lambda v: state()[0] == 1)
v80 = first + state()[1] - 80
log.write(f"phase seen at vsync {first} counter {state()[1]}: counter 80 at {v80}\n")
p.load_state(slot); time.sleep(0.5)
run(lambda v: v >= v80 - lat - 120)
p.save_state(8); time.sleep(1)
for k in ks:
    at = v80 + 80 - k
    p.load_state(8); time.sleep(0.3)
    v = run(lambda v: v >= at - lat)
    subprocess.run([os.path.join(T, "screenshot.sh"), os.path.join(out, f"k{k:+03}.png")], check=False, stdout=subprocess.DEVNULL)
    log.write(f"k {k} target vsync {at} asked {v} back {p.read32(VSYNC)} {state()}\n"); log.flush()
print("done")
