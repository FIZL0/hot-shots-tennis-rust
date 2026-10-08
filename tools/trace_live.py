#!/usr/bin/env python3
"""Record the live ball object every frame over PINE, from the next shot on.
Usage: trace_live.py <state_slot> <frames> <out.bin> [--net]
--net: right after the shot, aim the live ball low into the net (writes its velocity and zeroes its spin once; everything after
is the game's own physics). Output: records of (u32 frame since shot, 0x290-byte ball object), one per
distinct frame (polling can skip frames; the frame field says which). Only loads the slot; never saves."""
import struct, sys, time
from pine import Pine

GM_PTR, BALL, SIZE = 0x422f80, 0x98, 0x290
POS, VEL, FRAME, SPIN = 0xe0, 0x130, 0xac, 0x1a4

p = Pine(); p.require_realtime()  # polls by wall-clock: a copy launched with HST_REALTIME=1
p.load_state(int(sys.argv[1]))
time.sleep(0.2)
want, out, net = int(sys.argv[2]), open(sys.argv[3], "wb"), "--net" in sys.argv
last, armed, n, deadline = None, False, 0, time.time() + 60
ball = p.read32(p.read32(GM_PTR) + BALL)            # the live ball object stays put; one message per poll
while n < want and time.time() < deadline:
    blk = p.read_block(ball, SIZE)
    frame = struct.unpack_from("<I", blk, FRAME)[0]
    if not armed:
        if last is not None and frame < last and frame <= 2:   # a new shot started
            armed = True
            if net:
                x, y, z = struct.unpack_from("<3f", blk, POS)
                t, g = 20.0, 0.0027222224 * 0.9
                vel = (-x / t * 0.2, (-0.6 - y - 0.5 * g * t * t) / t, -z / t)
                f2u = lambda v: struct.unpack("<I", struct.pack("<f", v))[0]
                for k, v in enumerate(vel):
                    p.write32(ball + VEL + 4 * k, f2u(v))
                p.write32(ball + SPIN, 0)
                last = None
                continue
        last = frame
        continue
    if frame != last:
        out.write(struct.pack("<I", frame) + blk)
        last, n = frame, n + 1
print(n, "frames recorded" if armed else "no shot seen")
