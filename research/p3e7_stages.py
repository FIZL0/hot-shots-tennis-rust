#!/usr/bin/env python3
"""P3e7: which internal court (0x422f90) each Select Court entry starts: for each move count n (right ×n from the
list's first cursor, negative = left), slot 9 → pick → start the match → read the court."""
import os, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad

p = Pine()
for n in map(int, sys.argv[1:]):
    p.load_state(9); time.sleep(1.5)
    b = "right" if n > 0 else "left"
    vpad("press down 150", "sleep 600", "press cross 150", "sleep 2000", *[f"press {b} 150", "sleep 700"] * abs(n),
         "press cross 150", "sleep 2000", "press up 150", "sleep 600", "press cross 150", "sleep 15000")
    print(n, "court", p.read32(0x422f90), flush=True)
