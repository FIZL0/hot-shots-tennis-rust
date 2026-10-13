#!/usr/bin/env python3
"""B24d: from <slot> (court 4: 3, P1 human + 3 bots), press ○ every 1.5 s (P1 serves and swings) until the games total turns odd
and the match is about to change ends, then save scratch slot <save> there (for research/b24_rec15.py -<save>).
Usage (under tools/pcsx2.sh): b24d_ends.py <slot> <save>"""
import os, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine

GM = 0x422f80
VPAD = os.path.join(os.path.dirname(__file__), "..", "tools", "vpad.py")
slot, save = int(sys.argv[1]), int(sys.argv[2])
p = Pine()
p.load_state(slot)
time.sleep(1.0)
games = lambda: sum(p.read32(a) for a in (0x42306c, 0x423070))
g0, t, press = games(), time.time(), 0
while time.time() - t < 600:
    gm = p.read32(GM)
    st = p.read8(gm + 0x54) if gm else None
    if games() != g0 and games() % 2 == 1 and st in (3, 4, 5):
        p.save_state(save)
        print("saved", save, "games", games(), "state", st, flush=True)
        break
    if time.time() - press > 1.5:
        subprocess.run([VPAD, "send", "press circle 100"], check=True)
        press = time.time()
    time.sleep(0.05)
else:
    sys.exit("no change of ends in 10 min")
