#!/usr/bin/env python3
"""Record the umpire every frame from a save-state load (bot games: slot 5).
Usage: record_umpire.py <slot> <frames> <out.bin>. Sample (0x1d4 bytes; PINE reads in 8s): u32 vsync, gm+0x50 8 bytes (phase +0x55),
gm+0x340 8 bytes, umpire object +0xd0..+0x1b0, scoreboard +0x190 8 bytes (+0x192 shown) and +0x424 8 bytes
(+0x426 call), 0x423040..0x4230c0 (score), 0x316600..0x316630 (deuce/advantage/tiebreak), the ball +0xe0 16 bytes,
0x2ef7dc 8 bytes (umpire ids +2/+3) and 0x2ef110 8 bytes (language)."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, UMP_PTR, SB_PTR = 0x1d5780, 0x422f80, 0x43b1c8, 0x42d6c0
p, want, out = Pine(), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(int(sys.argv[1]))
time.sleep(0.3)
gm, ump, sb = p.read32(GM_PTR), p.read32(UMP_PTR), p.read32(SB_PTR)
r = [(gm + 0x50, 8), (gm + 0x340, 8), (ump + 0xd0, 0xe0), (sb + 0x190, 8), (sb + 0x424, 8), (0x423040, 0x80),
     (0x316600, 0x30), (p.read32(gm + 0x88) + 0xe0, 16), (0x2ef7dc, 8), (0x2ef110, 8)]
last, n = p.read32(VSYNC), 0
while n < want:
    v = p.read32(VSYNC)
    if v == last:
        continue
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    time.sleep(0.009)  # read late in the frame, once the game's tick is done
    a = p.read_regions(r)
    while True:  # the EE runs while PINE reads: re-read until two reads agree within the frame
        b = p.read_regions(r)
        if a == b and p.read32(VSYNC) == v: break
        a = b
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
