#!/usr/bin/env python3
"""P17k: start a doubles match from save slot 2 (tools/pick.py) and record what the weather schedule is drawn from.
Writes context/p17k/schedule.json: newlib rand() state (reent +0xa8) right after loading slot 2 and once the match
is up, the match's MT seed (0x42303c), court/games/sets/players, and the schedule (gm+0xd8 weathers, gm+0x11c
speed/degree pairs) as raw hex. Run under tools/pcsx2.sh."""
import json, os, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
import pick

p = Pine()
orig = p.load_state
before = {}
def load(slot):
    orig(slot)
    before["rand"] = p.read64(p.read32(0x1b80f0) + 0xa8)
p.load_state = load
pick.pick(p, int(sys.argv[1]) if len(sys.argv) > 1 else 0, 9)
gm = p.read32(0x422f80)
out = dict(rand_before=before["rand"], rand_after=p.read64(p.read32(0x1b80f0) + 0xa8), seed=p.read32(0x42303c),
           court=p.read32(0x422f90), players=p.read32(0x422fa4), games=p.read32(0x423044), sets=p.read32(0x423048),
           weather=p.read_block(gm + 0xd8, 65).hex(), wind=p.read_block(gm + 0x11c, 65 * 8).hex())
os.makedirs("context/p17k", exist_ok=True)
json.dump(out, open("context/p17k/schedule.json", "w"), indent=1)
print(out)
