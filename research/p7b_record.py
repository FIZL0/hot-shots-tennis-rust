#!/usr/bin/env python3
"""P7b: pick character <n> as P1 on the doubles character select (save slot 2), start the match, save the serve
moment to scratch slot 8, dump player 0's identity/hand/pelvis rows, then record the vpad run script (`RUN`) from
slot 8 with tools/record_p2m2.py. Run under tools/pcsx2.sh (HST_PCSX2=N copy).
Usage: p7b_record.py <char> <out dir> [frames]   → <dir>/p7b_cNN.bin (record_p2m2 samples), p7b_cNN_pelvis.bin
(player 0's 48 pelvis rows, +0x6b0 + m·0x40), p7b_cNN.txt (char, hand, model flag, selection record)."""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "../tools"))
from pine import Pine

# cursor path from Ashley (the cursor's start in slot 2) on the doubles grid
PATH = {0: [], 1: ["right"], 2: ["right"] * 2, 5: ["right"] * 3, 13: ["down"], 4: ["down", "right"],
        3: ["down", "right", "right"], 6: ["down", "right", "right", "right"], 7: ["down", "right", "down"],
        12: ["down", "right", "down", "right"], 8: ["down"] * 2, 9: ["down"] * 2 + ["right"],
        10: ["down"] * 2 + ["right"] * 2, 11: ["down"] * 2 + ["right"] * 3}
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
T = os.path.join(os.path.dirname(__file__), "../tools")

def vpad(*c):
    # the pad server queues commands: wait out the batch so the next step starts after it
    subprocess.run([f"{T}/vpad.py", "send", *c], check=True)
    time.sleep(sum(int(l.split()[-1]) for l in c if l.split()[0] in ("sleep", "press")) / 1000 + 0.1)

ch, out = int(sys.argv[1]), sys.argv[2]
frames = sys.argv[3] if len(sys.argv) > 3 else "1000"
p = Pine()
p.load_state(2)
time.sleep(1.5)
vpad("sleep 2500", *[c for d in PATH[ch] for c in (f"press {d} 150", "sleep 700")], "press circle 150", "sleep 1200",
     *["press circle 150", "sleep 1200"] * 3, "press start 150", "sleep 3000")
t = time.time()
while True:
    gm = p.read32(0x422f80)
    if not gm and time.time() - t < 20:
        vpad("press circle 150", "sleep 2500")  # "Start the match!" (the confirm screen takes a while to take input)
    pl = gm and p.read32(gm + 0xa8)
    if pl and p.read8(gm + 0x55) == 2 and p.read8(pl + 0x3fa4) == 1:
        break
    if time.time() - t > 90: sys.exit("match didn't reach the serve")
    time.sleep(0.1)
time.sleep(1)
p.save_state(8)
time.sleep(2)
mdl = p.read32(p.read32(p.read32(pl + 0x54)))
info = dict(char=p.read32(pl + 0x12bc), hand=struct.unpack("<f", struct.pack("<I", p.read32(pl + 0x12b4)))[0],
            flag=p.read8(mdl + 0x135), sel=p.read_block(0x2ef7f0, 0x10).hex()[8:], state=p.read8(pl + 0x3fa4))
print(info, flush=True)
tag = f"{out}/p7b_c{ch:02d}"
open(tag + ".txt", "w").write(f"{info}\n")
open(tag + "_pelvis.bin", "wb").write(p.read_block(pl + 0x6b0, 48 * 0x40))
if info["char"] != ch: sys.exit(f"picked character {info['char']}, not {ch}")
if "--menu" in sys.argv: sys.exit(0)
del p  # one PINE user at a time: the recorder opens its own
rec = subprocess.Popen([sys.executable, f"{T}/record_p2m2.py", "8", tag + ".bin", frames])
time.sleep(1.0)
vpad(*RUN)
sys.exit(rec.wait())
