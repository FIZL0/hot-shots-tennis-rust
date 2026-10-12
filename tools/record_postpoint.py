#!/usr/bin/env python3
"""B48: the between-points camera timeline from a save state. Each frame where anything below changes, prints
vsync, match phase gm+0x55, camera mode ch+0x51 (0 = match camera, else the cut-away shot), pending mode ch+0x50,
shot-camera *(ch+0x64) shot +0x114 / on +0x174, replay flag gm+0x344, replay count gm+0x358, game-end 0x4230b8,
games 0x42306c/70. Usage: record_postpoint.py <slot> <frames> [--press]: --press taps Cross every second during the
serve and rally (a human P1 in slots 3/4 serves and swings), never between points (a press there skips); --every
prints every frame of the point-over and change-ends phases and the serve's first 6 ticks, with next phase gm+0x56."""
import struct, sys, time
sys.path.insert(0, 'tools')
from pine import Pine
import vpad

VS, GM = 0x1d5780, 0x422f80
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(0.5)
last, prev = p.read32(VS), None
for n in range(int(sys.argv[2])):
    v = last = p.next_frame(last)
    gm = p.read32(GM)
    if not gm: continue
    ch = p.read32(gm + 0xbc); cam = p.read32(ch + 0x64)
    g = p.read_regions([(gm + 0x50, 0x10), (ch + 0x50, 8), (cam + 0x110, 8), (cam + 0x170, 8), (gm + 0x340, 0x20),
                        (0x4230b8, 8), (0x423068, 0x10)])
    tick = struct.unpack_from('<I', g, 8)[0]
    k = (g[5], g[16 + 1], g[16], g[24 + 4], g[32 + 4], g[40 + 4], struct.unpack_from('<i', g, 40 + 0x18)[0],
         struct.unpack_from('<i', g, 72)[0], struct.unpack_from('<ii', g, 80 + 4))
    if '--press' in sys.argv and g[5] in (2, 3) and n % 60 == 0: vpad.send(['press cross 100'])
    every = '--every' in sys.argv and (g[5] in (1, 4) or g[5] == 2 and tick < 6)
    if k != prev or every:
        print(v, 'tick', tick, 'next %d' % g[6], 'phase %d mode %#x pend %#x shot %#x on %d replay %d count %d ge %d games %s' % k, flush=True)
        prev = k
