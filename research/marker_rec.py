"""P19: per-vsync log of the head markers' show flag against the match phase and the HUD panel.
Prints vsync, gm phase (+0x55), phase ticks (+0x58), HUD mode (+0x148), panel slide (+0x164), marker flag (+0x725 of the
balloon manager, found by its vtable). First line: the marker colour tables (singles, doubles) and the camera fov.
usage: marker_rec.py <slot> <vsyncs>"""
import sys, time, struct
sys.path.insert(0, "tools")
from pine import Pine


def find(p):
    """The balloon manager: the one object whose vtable word is 0x1d1cc0."""
    for base in range(0x400000, 0x2000000, 0x10000):
        b = p.read_block(base, 0x10000)
        i = b.find(struct.pack("<I", 0x1d1cc0))
        while i >= 0:
            if i % 4 == 0:
                return base + i
            i = b.find(struct.pack("<I", 0x1d1cc0), i + 1)


p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
gm = p.read32(0x422f80); bm = find(p); hud = p.read32(0x42d6c0)
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
print("cols2", [p.read32(0x43b1f0 + 4 * k) for k in range(6)], "cols4", [p.read32(0x43b210 + 4 * k) for k in range(12)],
      "fov", f(0x1e7d50), "players", p.read32(0x422fa4), "slots", [p.read32(0x422fe8 + 4 * k) for k in range(4)],
      "chars", [p.read32(0x422fa8 + 4 * k) for k in range(4)], "y", [f(bm + 0x72c + 4 * k) for k in range(4)], flush=True)
last, n, prev = p.read32(0x1d5780), 0, None
while n < int(sys.argv[2]):
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    g = p.read_block(gm + 0x54, 8); h = p.read_block(hud + 0x148, 0x20)
    row = (g[1], h[0], struct.unpack("<i", h[0x1c:0x20])[0], p.read_block(bm + 0x724, 4)[1])
    if row != prev:
        print(v, struct.unpack("<I", g[4:8])[0], *row, flush=True)
    prev = row
