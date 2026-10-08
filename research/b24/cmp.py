#!/usr/bin/env python3
"""B24: compare two per-frame captures (u32 vsync + payload, fixed size) on the vsyncs both have.
Usage: cmp.py <size> <a.bin> <b.bin>  → frames in common, gaps in each, frames whose payload differs."""
import struct, sys
n = int(sys.argv[1])
def load(f):
    b = open(f, "rb").read()
    return {struct.unpack_from("<I", b, i)[0]: b[i + 4:i + n] for i in range(0, len(b) - n + 1, n)}
a, b = load(sys.argv[2]), load(sys.argv[3])
gaps = lambda d: sum(1 for v in range(min(d), max(d)) if v not in d)
common = sorted(a.keys() & b.keys())
diff = [v for v in common if a[v] != b[v]]
print(f"common {len(common)} ({common[0]}..{common[-1]}), gaps a={gaps(a)} b={gaps(b)}, differ {len(diff)}", diff[:10])
