#!/usr/bin/env python3
"""P7d: record the server's baseline walk. Run under tools/pcsx2.sh (HST_PCSX2=N copy at NominalScalar 0.5, so
1 s of script ≈ 30 frames).
  p7d_record.py prep        slot 4 → its next point's serve (P1 Carol serving, sub-state 0) saved to scratch 8, and
                            the same with player 0's hand (+0x12b4) poked to −1 saved to 9 (the walk reads the hand
                            only to pick its motion)
  p7d_record.py <slot> <out.bin> [frames=1300]   record_p2m2 from <slot> while vpad plays `WALK` twice"""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "../tools"))
from pine import Pine
from pick import vpad

# out to the far limit, in to the centre-mark limit, partial stick, the dead square, diagonals (only an x-dominant
# one walks), d-pad; then serve (the first press after a load or point is ignored, then toss, then hit)
SERVE = ["press cross 100", "sleep 1500"] * 3
WALK = ["sleep 400", "stick l 1 0", "sleep 3000", "stick l -1 0", "sleep 5000", "stick l 0.5 0", "sleep 800",
        "stick l 0.3 0", "sleep 600", "stick l 0.6 -0.8", "sleep 800", "stick l 0.8 0.6", "sleep 800",
        "stick l -0.7 0.69", "sleep 800", "release", "down right", "sleep 800", "up right", "down left", "sleep 600",
        "down up", "sleep 600", "release", *SERVE]
T = os.path.join(os.path.dirname(__file__), "../tools")

if sys.argv[1] == "prep":
    p = Pine()
    p.load_state(4)
    t = time.time()
    seen_play = False
    while time.time() - t < 60:
        gm = p.read32(0x422f80)
        pl = p.read32(gm + 0xa8)
        st, sub = p.read8(pl + 0x3fa4), p.read8(pl + 0x3fa6)
        seen_play |= st != 1
        if seen_play and st == 1 and sub == 0:
            break
        time.sleep(0.05)
    else:
        sys.exit("no next serve")
    time.sleep(1)
    p.save_state(8)
    time.sleep(2)
    print("slot 8: x", struct.unpack("<f", struct.pack("<I", p.read32(pl + 0x3d70)))[0], "hand",
          struct.unpack("<f", struct.pack("<I", p.read32(pl + 0x12b4)))[0])
    p.write32(pl + 0x12b4, 0xbf800000)
    p.save_state(9)
    time.sleep(2)
    sys.exit(0)

slot, out = sys.argv[1], sys.argv[2]
frames = sys.argv[3] if len(sys.argv) > 3 else "1300"
rec = subprocess.Popen([sys.executable, f"{T}/record_p2m2.py", slot, out, frames])
time.sleep(1.0)
vpad(*WALK, "sleep 6000", *WALK)
sys.exit(rec.wait())
