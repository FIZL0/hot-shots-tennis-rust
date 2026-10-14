#!/usr/bin/env python3
"""B273: frame-time summary of tools/web_check.mjs's OUT/framesN.json ([ms since load, frame ms] per rAF), from FROM s on."""
import json, sys
f, start = sys.argv[1], float(sys.argv[2]) if len(sys.argv) > 2 else 0
d = sorted(ms for t, ms in json.load(open(f)) if t >= start * 1000)
if not d: sys.exit(f"{f}: no frames")
n = len(d)
print(f"{f}: {n} frames, mean {sum(d)/n:.2f} ms ({1000*n/sum(d):.0f} fps), median {d[n//2]:.2f}, p95 {d[int(n*.95)]:.2f}, max {d[-1]:.1f}")
