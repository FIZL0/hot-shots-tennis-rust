#!/usr/bin/env python3
"""P3f4: a doubles match where all four players have character 1, saved at P1's first serve.
From save slot 2 (doubles character select; cursors start P1 0, 2P 2, 3P 1, 4P 5 on the BEG row 0 1 2 5) each
cursor walks onto character 1 and confirms, then the match starts as tools/pick.py's.
Usage (under tools/pcsx2.sh): p3f4_four_of_one.py <save slot> [confirm button, default circle] [extra frames to wait
in the menu, varies the seed]. Then research/p3d3_setup_ram.py --chars over the saved state."""
import os, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad

slot = int(sys.argv[1])
ok = sys.argv[2] if len(sys.argv) > 2 else "circle"
wait = int(sys.argv[3]) if len(sys.argv) > 3 else 0
p = Pine()
p.load_state(2)
time.sleep(1.5)
step = lambda *d: [c for x in d for c in (f"press {x} 150", "sleep 700")]
vpad(f"sleep {2500 + wait}", *step("right"), f"press {ok} 150", "sleep 1200", *step("left"), f"press {ok} 150",
     "sleep 1200", f"press {ok} 150", "sleep 1200", *step("left", "left"), f"press {ok} 150", "sleep 1200",
     "press start 150", "sleep 3000")
t = time.time()
while True:
    gm = p.read32(0x422f80)
    if not gm and time.time() - t < 20:
        vpad(f"press {ok} 150", "sleep 2500")
    pl = gm and p.read32(gm + 0xa8)
    if pl and p.read8(gm + 0x55) == 2 and p.read8(pl + 0x3fa4) == 1:
        break
    if time.time() - t > 90: sys.exit("match didn't reach the serve")
    time.sleep(0.1)
chars = [p.read32(0x422fa8 + 4 * k) for k in range(4)]
p.save_state(slot)
time.sleep(2)
print("characters", chars, f"saved to slot {slot}")
