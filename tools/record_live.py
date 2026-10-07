#!/usr/bin/env python3
"""Record the live ball every frame from a save-state load (bot games: slot 5).
Usage: record_live.py <slot> <frames> <out.bin> [--aim x,y,z,t[,serve]]. Run PCSX2 slowed down ([Framerate]
NominalScalar = 0.25) so every frame is caught; a sample is taken only when two reads in a row agree (never torn
mid-frame).
Sample = u32 vsync + live ball *(gm+0x88) 0x290 + predictor *(gm+0x98) 0x290 + rally block 0x3165f0 0x40.
--aim: at the first shot (or serve, with `serve`) after the load, rewrite the live ball's velocity once so it would
reach game point (x, y, z) in t frames under gravity alone (net cord, post, net tests); everything after is the
game's own physics. The vsync of the sample before the write goes to <out.bin>.poke (replays skip the next frames)."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, G = 0x1d5780, 0x422f80, 0.0027222224 * 0.9
POS, VEL, FRAME, CLASS = 0xe0, 0x130, 0xac, 0x58
p, want, out = Pine(), int(sys.argv[2]), open(sys.argv[3], "wb")
aim = next((a.split(",") for a in sys.argv[4:] if a.count(",") >= 3), None) if "--aim" in sys.argv else None
p.load_state(int(sys.argv[1]))
time.sleep(0.3)
gm = p.read32(GM_PTR)
ball = p.read32(gm + 0x88)
r = [(ball, 0x290), (p.read32(gm + 0x98), 0x290), (0x3165f0, 0x40)]
last, n, prev = p.read32(VSYNC), 0, None
while n < want:
    v = p.read32(VSYNC)
    if v == last:
        continue
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    time.sleep(0.004)
    a = p.settle(r, v)
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
    frame, moving = struct.unpack_from("<i", a, FRAME)[0], struct.unpack_from("<3f", a, VEL) != (0.0, 0.0, 0.0)
    serve = a[CLASS] == 0
    if aim and prev is not None and frame < prev and frame <= 2 and moving and (len(aim) > 4) == serve:
        x, y, z = struct.unpack_from("<3f", a, POS)
        tx, ty, tz, t = map(float, aim[:4])
        vel = ((tx - x) / t, (ty - y - 0.5 * G * t * t) / t, (tz - z) / t)
        for k, c in enumerate(vel):
            p.write32(ball + VEL + 4 * k, struct.unpack("<I", struct.pack("<f", c))[0])
        open(sys.argv[3] + ".poke", "w").write(f"{v}\n")
        print(f"aimed at vsync {v} from {x:.2f},{y:.2f},{z:.2f}", flush=True)
        aim = None
    prev = frame
print("done", n)
