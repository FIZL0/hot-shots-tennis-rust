#!/usr/bin/env python3
"""P7b: start a match with character <n> as P1 (tools/pick.py: the serve saved to scratch slot 8), dump player 0's identity/hand/pelvis rows, then record the vpad run script (`RUN`) from
slot 8 with tools/record_p2m2.py. Run under tools/pcsx2.sh (HST_PCSX2=N copy).
Usage: p7b_record.py <char> <out dir> [frames]   → <dir>/p7b_cNN.bin (record_p2m2 samples), p7b_cNN_pelvis.bin
(player 0's 48 pelvis rows, +0x6b0 + m·0x40), p7b_cNN.txt (char, hand, model flag, selection record)."""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "../tools"))
from pine import Pine
from pick import pick, vpad

# serve (toss, hit), then stick/d-pad runs while the rally lasts
# Each point is short (P1 serves, the rally ends in ~2 s), so serve and run four times. Serve: the first press after
# a load or a point is ignored, the next tosses, the third hits. A press left over in play is a swing (not stepped).
SERVE = ["press circle 100", "sleep 1500"] * 3
RUN = ["sleep 200", *SERVE, "stick l -1 0", "sleep 900", "stick l 1 0", "sleep 900", "stick l 0 -1", "sleep 600", "release",
       "sleep 2000", *SERVE, "stick l 0.7 0.7", "sleep 700", "stick l -0.5 0.2", "sleep 700", "stick l 0.2 0", "sleep 400",
       "stick l 0 1", "sleep 600", "release",
       "sleep 2000", *SERVE, "down left", "down up", "sleep 600", "up left", "up up", "down right", "sleep 600",
       "up right", "down left", "sleep 600", "release",
       "sleep 2000", *SERVE, "stick l -0.9 -0.4", "sleep 800", "stick l 1 1", "sleep 700", "stick l 0.1 0.05",
       "sleep 400", "stick l -1 1", "sleep 600", "release"]
# a point's length varies and a serve sequence can land in play and be lost: two more cycles (record 1500 frames)
RUN += ["sleep 2000", *SERVE, "stick l 1 0", "sleep 900", "stick l -0.6 -0.6", "sleep 900", "release",
        "sleep 2000", *SERVE, "stick l 0 -1", "sleep 700", "stick l 0.4 0.9", "sleep 900", "release"]
T = os.path.join(os.path.dirname(__file__), "../tools")

ch, out = int(sys.argv[1]), sys.argv[2]
frames = sys.argv[3] if len(sys.argv) > 3 else "1500"
p = Pine()
pl, info = pick(p, ch)
print(info, flush=True)
tag = f"{out}/p7b_c{ch:02d}"
open(tag + ".txt", "w").write(f"{info}\n")
open(tag + "_pelvis.bin", "wb").write(p.read_block(pl + 0x6b0, 48 * 0x40))
if "--menu" in sys.argv: sys.exit(0)
del p  # one PINE user at a time: the recorder opens its own
rec = subprocess.Popen([sys.executable, f"{T}/record_p2m2.py", "8", tag + ".bin", frames])
time.sleep(1.0)
vpad(*RUN)
sys.exit(rec.wait())
