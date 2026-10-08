"""Record the ball (slot 5) per frame: vsync, position x y z w and its model light scale (+0x128), as hex bits.
Usage: tools/pcsx2.sh python3 research/p17t_ball_rec.py <frames> > context/p17t/ball05.txt"""
import sys, struct
sys.path.insert(0, "tools")
from pine import Pine, VSYNC
p = Pine(step=True); p.load_state(5)
import time; time.sleep(0.3)
v = p.read32(VSYNC); o = 0xc9f2a0
mdl = p.read32(p.read32(o + 0x274)); print("# mdl", hex(mdl), flush=True)
for _ in range(int(sys.argv[1])):
    v = p.next_frame(v)
    b = p.settle([(o + 0xe0, 0x10), (mdl + 0x128, 8)], v)
    if b is None: continue
    print(v, " ".join(f"{w:08x}" for w in struct.unpack("<5I", b[:20])), flush=True)
