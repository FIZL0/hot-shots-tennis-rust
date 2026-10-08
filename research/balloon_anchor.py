"""B21: per-vsync log of each player's pop-up anchor node (player obj +0x54 → +0x84, matrix translation +0x30; game
y is down-positive) and the pop-up list entries (as surprise_rec.py). Finds the anchor node's index in the model's
node matrices by matching the translation.
usage: balloon_anchor.py <slot> <vsyncs>"""
import sys, time, struct
sys.path.insert(0, "tools")
from pine import Pine


def find(p):
    for base in range(0x400000, 0x2000000, 0x10000):
        b = p.read_block(base, 0x10000)
        i = b.find(struct.pack("<I", 0x1d1cc0))
        while i >= 0:
            if i % 4 == 0:
                return base + i
            i = b.find(struct.pack("<I", 0x1d1cc0), i + 1)


p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
gm = p.read32(0x422f80); n = p.read32(0x422fa4); bm = find(p)
pl = [p.read32(gm + 0xa8 + 4 * i) for i in range(n)]
fl = lambda b, o, k=1: struct.unpack_from("<%df" % k, b, o)
print("players", n, "bm", hex(bm), "chars", [p.read32(0x422fa8 + 4 * k) for k in range(n)])
for i, a in enumerate(pl):
    mdl = p.read32(a + 0x54); node = p.read32(mdl + 0x84)
    print(i, "obj", hex(a), "mdl", hex(mdl), "node", hex(node), "mdl words", [hex(p.read32(mdl + 4 * k)) for k in range(0x30)])
last, k = p.read32(0x1d5780), 0
while k < int(sys.argv[2]):
    v = p.read32(0x1d5780)
    if v == last: continue
    last, k = v, k + 1
    row = []
    for a in pl:
        node = p.read32(p.read32(a + 0x54) + 0x84)
        row.append("(%.3f %.3f %.3f)" % fl(p.read_block(node + 0x30, 12), 0, 3))
    cnt = min(p.read32(bm + 0x6f0), 15)
    blk = p.read_block(bm + 0x150, 0x30 * cnt) if cnt else b""
    pops = []
    for j in range(cnt):
        e = blk[0x30 * j:0x30 * j + 0x30]
        pops.append("k%d s%d p%d st%d t%d a%.0f" % (e[0x29], e[0x1c], struct.unpack_from("<i", e, 0x20)[0], struct.unpack_from("<b", e, 0x14)[0], struct.unpack_from("<i", e, 0x10)[0], fl(e, 0x18)[0]))
    print(k, v, " ".join(row), "|", "; ".join(pops), flush=True)
