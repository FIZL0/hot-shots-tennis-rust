#!/usr/bin/env python3
"""Capture every shot's starting ball state and its precomputed path over PINE.
Usage: trace_shots.py <state_slot> <seconds> <out_dir>
Writes <out_dir>/shot_NNN.ball (0x290-byte ball object at path frame 0, plus its params block) and
shot_NNN.path (n × 0x30 entries). Only loads the slot; never saves states."""
import os, struct, sys, time
from pine import Pine

GM_PTR = 0x422f80                    # game manager pointer (US 1.00)
BALL, PATH = 0x98, 0xa4              # gm offsets: live ball sim, stored path record
BALL_SIZE, PARAMS_PTR, PARAMS_SIZE = 0x290, 0x54, 0x900

p = Pine()
p.load_state(int(sys.argv[1]))
time.sleep(0.2)
os.makedirs(sys.argv[3], exist_ok=True)
end, shots, cur = time.time() + float(sys.argv[2]), 0, None   # cur = [base, ball_blob, path, last_frame]

def flush(c):
    if c and c[2] and len(c[2]) >= 2 * 0x30:
        open(c[0] + ".ball", "wb").write(c[1])
        open(c[0] + ".path", "wb").write(c[2])
        return 1
    return 0

while time.time() < end:
    gm = p.read32(GM_PTR)
    ball = p.read32(gm + BALL)
    blk = p.read_block(ball, BALL_SIZE)
    frame = struct.unpack_from("<I", blk, 0xac)[0]
    rec = p.read32(gm + PATH)
    arr, n = p.read32(rec + 0x50), p.read32(rec + 0x54)
    if cur is None or frame < cur[3]:               # frame counter restarted: a new shot
        shots += flush(cur)
        cur = None
        if frame <= 1 and 0 < n <= 180:
            params = p.read_block(struct.unpack_from("<I", blk, PARAMS_PTR)[0], PARAMS_SIZE)
            cur = [os.path.join(sys.argv[3], f"shot_{shots:03d}"), struct.pack("<I", frame) + blk + params, b"", frame]
    if cur is not None:
        cur[3] = frame
        if 0 < n <= 180:
            cur[2] = p.read_block(arr, n * 0x30)       # the game fills the path over several frames; keep latest
shots += flush(cur)
print(shots, "shots captured")
