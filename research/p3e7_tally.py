#!/usr/bin/env python3
"""P3e7: tally p3e7_court5.py's log: per point over (the last sample of the vsync entering phase 4), the call,
point kind and 0x4230b8 against whether court 5's 93 marks went up, and the decomp's prediction."""
import sys
from collections import Counter
d = {}
for r in map(str.split, open(sys.argv[1])):
    if r[2].endswith("→4") and r[2] != "4→4":
        d[r[0]] = tuple(int(r[k]) for k in (10, 12, 14, 8))
c = Counter()
for call, kind, b8, count in d.values():
    if call != 255:
        c[(call, kind, b8, count == 93, call == 0 and (b8 > 0 or kind in (1, 2)))] += 1
for k, n in sorted(c.items()):
    print("call %d kind560 %d b8 %d shown %s predicted %s: %d" % (k + (n,)))
