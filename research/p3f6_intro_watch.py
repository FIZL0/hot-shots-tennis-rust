#!/usr/bin/env python3
"""P3f6: watch the match intro's three gates (match phase 0 ends when all are done): the timer at
*(*(gm+0x84)+0x138)+0x3c, the streamed sound busy word (0x1e9f10) and, in doubles, the presentation's flags
(*0x43b1d8 + 0xd94/0xd95). Prints each change with the match frame gm+0x58 (polled, not lock-step: ±1 frame).
Usage (under tools/pcsx2.sh): p3f6_intro_watch.py <load slot> <vpad command>... (route as research/p3f2_intro_log.py).
P3F6_STAGE=n: write n over the court (0x422f90) as soon as the menu sets it, before the court loads (reaches courts
the Select Court list route doesn't; rewritten until the match object exists)."""
import os, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad

slot, route = int(sys.argv[1]), sys.argv[2:]
p = Pine()
p.load_state(slot)
time.sleep(1.5)
vpad(*route)
t, last = time.time(), None
stage = os.environ.get("P3F6_STAGE")
while stage and not p.read32(0x422f80) and time.time() - t < 60:
    if p.read32(0x422f90) not in (0, int(stage)): p.write32(0x422f90, int(stage))
while time.time() - t < 120:
    gm = p.read32(0x422f80)
    if not gm: time.sleep(0.05); continue
    o = p.read32(p.read32(gm + 0x84) + 0x138) if p.read32(gm + 0x84) else 0
    pr = p.read32(0x43b1d8)
    s = (p.read8(gm + 0x55), p.read8(gm + 0x56), o and p.read32(o + 0x3c), p.read32(0x1e9f10),
         pr and p.read8(pr + 0xd94), pr and p.read8(pr + 0xd95))
    key = s[:2] + (s[2] > 0 if s[2] < 1 << 31 else -1,) + s[3:]
    if key != last:
        print(f"frame {p.read32(gm + 0x58)} state/phase {s[0]}/{s[1]} obj {o:#x} timer {s[2]} sound {s[3]:#x}"
              f" pres {s[4]}/{s[5]} court {p.read32(0x422f90)} players {p.read32(0x422fa4)}", flush=True)
        last = key
    if s[0] == 2 and s[1] >= 2: break
