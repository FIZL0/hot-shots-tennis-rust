"""P3d2: research/p3d2_formation.py's pokes in lock-step, one JSON line per frame: vsync, gm+0x55, the main
generator's index and first 4 words, each player's +0x13f4 and +0x13f5. Usage: tools/pcsx2.sh python3
research/p3d2_frames.py <human −1..3> <frames> > context/fixtures/p3d2_frames_hN.jsonl"""
import json, sys, time
sys.path.insert(0, 'tools')
from pine import Pine, VSYNC

def poke8(p, a, v):
    w = p.read32(a & ~3)
    s = (a & 3) * 8
    p.write32(a & ~3, w & ~(0xff << s) | v << s)

human, frames = int(sys.argv[1]), int(sys.argv[2])
p = Pine(step=True)
p.load_state(5)
time.sleep(1)
gm = p.read32(0x422f80)
pls = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
ais = [p.read32(pl + 0x80) for pl in pls]
rows = [p.read32(ai + 0xc) for ai in ais]
if human >= 0:
    p.write32(0x422fc8 + 4 * human, 0)
poke8(p, rows[0] + 2, 1)
poke8(p, rows[2] + 2, 2)
for i, v in enumerate([1, 2, 3, 1]):
    poke8(p, ais[i] + 0x11, v)
mt = p.read32(gm + 0x80)
last = p.read32(VSYNC)
for _ in range(frames):
    last = p.next_frame(last)
    if p.read8(0x423040) == 0:
        poke8(p, 0x423040, 1)
    print(json.dumps({'vsync': last, 'phase': p.read8(gm + 0x55), 'index': p.read32(mt + 0x9c4),
        'words': [p.read32(mt + 4 + 4 * j) for j in range(4)], 'formation': [p.read8(pl + 0x13f4) for pl in pls],
        'skip': [p.read8(pl + 0x13f5) for pl in pls]}), flush=True)
