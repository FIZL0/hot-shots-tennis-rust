#!/usr/bin/env python3
"""B37: does the original reset a server's stance (+0x140c, where its last toss stood) at a change of ends?
Loads slot 5 (bot doubles), once the first rally is on pokes distinct stances on players 1..3 and team 0 to 40-0,
then prints every player's x and stance as each phase is entered up to the second serve set-up after the change."""
import struct, sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine

GM, VSYNC = 0x422f80, 0x1d5780
f32 = lambda v: struct.unpack("<f", struct.pack("<I", v))[0]
u32 = lambda x: struct.unpack("<I", struct.pack("<f", x))[0]
p = Pine()
p.load_state(int(os.environ.get("SLOT", 5)))
gm = lambda: p.read32(GM)
phase = lambda: p.read8(gm() + 0x55)
players = lambda: [p.read32(gm() + 0xa8 + 4 * i) for i in range(p.read32(0x422fa4))]
def show(tag):
    pl = players()
    print(tag, "v", p.read32(VSYNC), "phase", phase(), "srv", p.read32(0x42304c), "side", p.read32(0x423050),
          "games", p.read32(0x42306c), p.read32(0x423070),
          "x", [round(f32(p.read32(o + 0x3d70)), 4) for o in pl], "stance", [round(f32(p.read32(o + 0x140c)), 4) for o in pl],
          flush=True)
last, poked, changes, v = phase(), False, 0, 0
while True:
    v = p.next_frame(v)
    ph = phase()
    if ph != last:
        show(f"enter {ph}")
        if ph == 3 and not poked:
            for i, o in enumerate(players()[1:], 1):
                p.write32(o + 0x140c, u32(1.0 + 0.5 * i))
            p.write32(0x423064, 3)
            poked = True
            show("poked")
        if ph == 1:
            changes += 1
        if changes and ph == 3:
            break
        last = ph
