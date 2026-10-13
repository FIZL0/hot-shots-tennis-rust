#!/usr/bin/env python3
"""B98: from a save state, run to the first game-ending point and screenshot the original at post-point tick N
(gm+0x58, phase gm+0x55 = 4) of it; prints the cut-away shot number and its eye/forward. Usage:
b98_game_end_shot.py <slot> <out.png> [tick=120]"""
import struct, subprocess, sys, time
sys.path.insert(0, 'tools')
from pine import Pine

VS, GM = 0x1d5780, 0x422f80
want = int(sys.argv[3]) if len(sys.argv) > 3 else 120
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(0.5)
last, games = p.read32(VS), None
for _ in range(30000):
    last = p.next_frame(last)
    gm = p.read32(GM)
    if not gm: continue
    g = sum(struct.unpack('<ii', p.read_block(0x42306c, 8)))
    games = g if games is None else games
    if g == games or p.read8(gm + 0x55) != 4 or p.read32(gm + 0x58) < want: continue
    ch = p.read32(gm + 0xbc); cam = p.read32(ch + 0x64)
    v = struct.unpack('<16f', p.read_block(cam + 0x60, 0x40))
    print('vsync', last, 'shot %#x' % p.read8(cam + 0x114), 'mode %#x' % p.read8(ch + 0x51), 'fwd', v[8:11], 'eye', v[12:15], flush=True)
    subprocess.run(['tools/screenshot.sh', sys.argv[2]], check=True)  # live: a few frames after the tick
    break
