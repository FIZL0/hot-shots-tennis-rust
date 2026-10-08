#!/usr/bin/env python3
"""P3e7: from the confirm screen (slot 9) open Select Court and screenshot the list. Usage: p3e7_court.py [moves...]
where each move is a vpad button pressed in turn after opening the list (e.g. right right); then shot."""
import os, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad

p = Pine()
p.load_state(9); time.sleep(1.5)
vpad("press down 150", "sleep 600", "press cross 150", "sleep 2000",
     *[x for b in sys.argv[2:] for x in (f"press {b} 150", "sleep 700")])
subprocess.run(["tools/screenshot.sh", sys.argv[1]])
print("stage", p.read32(0x422f90))
