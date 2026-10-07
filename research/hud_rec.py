"""P19: per-vsync log of the in-match HUD's panel state against the match phase.
Prints vsync, gm phase (+0x55), phase ticks (+0x58), HUD mode (+0x148), slide count (+0x164), fade state (+0x150),
fade counter (+0x154), and the faces' alpha. usage: hud_rec.py <slot> <vsyncs>"""
import sys, time, struct
sys.path.insert(0, "tools")
from pine import Pine
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
last, n = p.read32(0x1d5780), 0
while n < int(sys.argv[2]):
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    gm, hud = p.read32(0x422f80), p.read32(0x42d6c0)
    g = p.read_block(gm + 0x54, 8); h = p.read_block(hud + 0x148, 0x20)
    face_a = struct.unpack("<f", struct.pack("<I", p.read32(p.read32(hud + 0x74) + 0x5c)))[0]
    print(v, g[1], struct.unpack("<I", g[4:8])[0], h[0], struct.unpack("<i", h[0x1c:0x20])[0], h[8],
          struct.unpack("<i", h[0xc:0x10])[0], face_a, flush=True)
