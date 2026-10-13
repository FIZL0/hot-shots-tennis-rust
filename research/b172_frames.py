#!/usr/bin/env python3
"""B172: the instant replay's reference frames. From save slot <slot> (just before a replay), every vsync while
a replay runs (gm+0x344): u32 vsync, u8 shot on (handler+0x51), u32 shot number (*(handler+0x64)+0x114), the scene
frame table *(gm+0xb8)+0xd130 (0x79 x 0x40), scene +0x10dc0 0x40 (record index, subjects) and +0x10e70 0x10
(pick snapshot: roles, subject, next hitter), +0x10f98 8 (live roles), the camera eye/look/up/fov 0x1e7d20 0x40. Usage: b172_frames.py <slot> <out.bin> <max>"""
import sys, time, struct
sys.path.insert(0, 'tools')
from pine import Pine
p = Pine(step=True); p.load_state(int(sys.argv[1])); time.sleep(1)
out = open(sys.argv[2], 'wb'); n = seen = 0
last = p.read32(0x1d5780)
for _ in range(int(sys.argv[3])):
    v = last = p.next_frame(last)
    gm = p.read32(0x422f80)
    if not gm: continue
    if not p.read8(gm + 0x344):
        if seen: break
        continue
    seen = 1
    ch, sc = p.read32(gm + 0xbc), p.read32(gm + 0xb8)
    on = p.read8(ch + 0x51); cam = p.read32(ch + 0x64)
    num = p.read32(cam + 0x114) if cam else 0xffffffff
    out.write(struct.pack('<IBI', v, on, num) + p.read_block(sc + 0xd130, 0x79 * 0x40) + p.read_block(sc + 0x10dc0, 0x40)
              + p.read_block(sc + 0x10e70, 0x10) + p.read_block(sc + 0x10f98, 8) + p.read_block(0x1e7d20, 0x40)); n += 1
print('done', n)
