"""B16: do the original's head markers depth-test? Loads a slot where they show (3: P1 serves and waits), screenshots,
then sinks every marker's anchor height (balloon manager +0x72c, game y is down-positive) under the court and
screenshots again: a marker still on screen under the ground means no depth test.
usage: marker_depth.py <slot> <out-prefix> [y]"""
import sys, time, struct, subprocess
sys.path.insert(0, "tools")
from pine import Pine


def find(p):
    """The balloon manager: the one object whose vtable word is 0x1d1cc0 (as marker_rec.py)."""
    for base in range(0x400000, 0x2000000, 0x10000):
        b = p.read_block(base, 0x10000)
        i = b.find(struct.pack("<I", 0x1d1cc0))
        while i >= 0:
            if i % 4 == 0:
                return base + i
            i = b.find(struct.pack("<I", 0x1d1cc0), i + 1)


p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1.5)
bm = find(p); n = p.read32(0x422fa4)
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
print("bm", hex(bm), "players", n, "flag", p.read8(bm + 0x725), "y", [f(bm + 0x72c + 4 * k) for k in range(n)], flush=True)
subprocess.run(["tools/screenshot.sh", f"{sys.argv[2]}_before.png"])
y = float(sys.argv[3]) if len(sys.argv) > 3 else 5.0
for k in range(n):
    p.write32(bm + 0x72c + 4 * k, struct.unpack("<I", struct.pack("<f", y))[0])
time.sleep(1.5)
print("y now", [f(bm + 0x72c + 4 * k) for k in range(n)], flush=True)
subprocess.run(["tools/screenshot.sh", f"{sys.argv[2]}_sunk.png"])
