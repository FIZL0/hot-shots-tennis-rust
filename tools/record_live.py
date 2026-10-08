#!/usr/bin/env python3
"""Record the live ball every frame from a save-state load (bot games: slot 5).
Usage: record_live.py <slot> <frames> <out.bin> [--vel vsync,vx,vy,vz]. A PCSX2 copy runs at 1x (tools/pine.py; HST_LOCKSTEP=1: every frame, slowly); the user's own PCSX2: run it slowed (NominalScalar 0.25).
Sample = u32 vsync + live ball *(gm+0x88) 0x290 + predictor *(gm+0x98) 0x290 + rally block 0x3165f0 0x40.
--vel: right after sampling that vsync, overwrite the live ball's velocity once (net cord, post and net tests;
pick it by stepping hst-sim from the same state); everything after is the game's own physics. That vsync goes to
<out.bin>.poke (replays skip the next frames); none is written if the sample was missed."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, VEL = 0x1d5780, 0x422f80, 0x130
p, want, out = Pine(step=True), int(sys.argv[2]), open(sys.argv[3], "wb")
poke = sys.argv[sys.argv.index("--vel") + 1].split(",") if "--vel" in sys.argv else None
p.load_state(int(sys.argv[1]))
time.sleep(0.3)
gm = p.read32(GM_PTR)
ball = p.read32(gm + 0x88)
r = [(ball, 0x290), (p.read32(gm + 0x98), 0x290), (0x3165f0, 0x40)]
last, n = p.read32(VSYNC), 0
while n < want:
    v = p.next_frame(last)
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    a = p.settle(r, v)
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
    if poke and v == int(poke[0]):
        for k, c in enumerate(poke[1:]):
            p.write32(ball + VEL + 4 * k, struct.unpack("<I", struct.pack("<f", float(c)))[0])
        open(sys.argv[3] + ".poke", "w").write(f"{v}\n")
        print(f"velocity written after vsync {v}", flush=True)
print("done", n)
