#!/usr/bin/env python3
"""P3f3: research/p3f_menu_log.py with court weather odds poked first, to see the setup's clouds in a first game that
isn't clear. Pokes ADDR=VALUE words (from HST_POKE, "0x41ce6c=100,0x41ce78=0") once, at the stub's resume (the
state has landed by then). Usage (under tools/pcsx2.sh): HST_POKE=... p3f3_setup_log.py <p3f_menu_log args>."""
import os, runpy, sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
import pine

pokes = [tuple(int(x, 0) for x in kv.split("=")) for kv in os.environ.get("HST_POKE", "").split(",") if kv]
resume = pine.Pine.resume


def poked(self):
    global pokes
    for a, v in pokes: self.write32(a, v)
    pokes = []
    return resume(self)


pine.Pine.resume = poked
sys.argv = [os.path.join(os.path.dirname(__file__), "p3f_menu_log.py")] + sys.argv[1:]
runpy.run_path(sys.argv[0], run_name="__main__")
