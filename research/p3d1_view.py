"""P3d1: the post-point reaction voice's view test inputs, slot 5 (doubles bots), lock-step. Every frame from the
state until <frames> have gone by, a record (PINE reads 8 bytes at a time, so 4-byte fields take 8): u32 vsync, then
players 0x422fa4 8, 0x4230a8 8, gm+0x35c 8, camera eye/look 0x1e7d20 0x20, view 0x1e7e30 0x40, proj 0x1e7ff0 0x40,
fov (degrees) 0x1e7d50 8, then per player (4): +0x3ba0 8, +0x3fa4 8, +0x3db0 8, +0x12b8 8, +0x17e0 0x10,
+0x3d70 0x10, *(+0x1808)+0x30 0x10, +0x3d38 8, +0x3b64 0x18.
Usage: tools/pcsx2.sh python3 research/p3d1_view.py <out.bin> <frames>"""
import struct, sys, time
sys.path.insert(0, 'tools')
from pine import Pine

VS = 0x1d5780
p = Pine(step=True)
p.load_state(5)
time.sleep(1)
out = open(sys.argv[1], 'wb')
want = int(sys.argv[2])
last = p.read32(VS)
n = 0
while n < want:
    v = last = p.next_frame(last)
    gm = p.read32(0x422f80)
    if not gm:
        continue
    pls = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
    r = [(0x422fa4, 4), (0x4230a8, 4), (gm + 0x35c, 4), (0x1e7d20, 0x20), (0x1e7e30, 0x40), (0x1e7ff0, 0x40), (0x1e7d50, 8)]
    for pl in pls:
        r += [(pl + 0x3ba0, 4), (pl + 0x3fa4, 4), (pl + 0x3db0, 4), (pl + 0x12b8, 8), (pl + 0x17e0, 0x10), (pl + 0x3d70, 0x10),
              (p.read32(pl + 0x1808) + 0x30, 0x10), (pl + 0x3d38, 4), (pl + 0x3b64, 0x18)]
    out.write(struct.pack('<I', v) + p.read_regions(r))
    n += 1
print('done', n)
