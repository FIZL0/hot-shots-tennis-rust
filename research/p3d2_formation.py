"""P3d2: the doubles formation pick at a match's first point (+0x13f4), slot 5 (four bots), in lock-step.
Pokes the first-point flag 0x423040 back to 1 every frame so every new point picks again, makes player <human> a
human in the controller words 0x422fc8 + 4·i (0, as pad 1; −1: none), sets byte 2 of players 0 and 2's AIParam
rows (1, 2) and the AI objects' setup byte +0x11 (players 0..3: 1, 2, 3, 1). At each new point (the main
generator's index drops: the reseed and its four placement draws) it
writes a JSON line: server, receiver, gm+0x55, the generator's index and first 4 words, the controller words, setup
and row bytes, each player's +0x13f4 and +0x13f5, gm+0x344 and the player pointers; then it sets every +0x13f4 to
0x77, so a pick that didn't run (the keep-placement flag +0x13f5) shows as 0x77 next point.
Usage: tools/pcsx2.sh python3 research/p3d2_formation.py <human −1..3> <points> > context/fixtures/p3d2_formation_hN.jsonl"""
import json, struct, sys, time
sys.path.insert(0, 'tools')
from pine import Pine, VSYNC

def poke8(p, a, v):  # PINE's 8-bit write: a 32-bit read-modify-write races the game's own byte writes
    p._call(4, struct.pack('<IB', a, v))


human, points = int(sys.argv[1]), int(sys.argv[2])
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
last, n, t, vs = p.read32(mt + 0x9c4), 0, time.monotonic(), p.read32(VSYNC)
while n < points and time.monotonic() - t < 1000:
    vs = p.next_frame(vs)
    if p.read8(0x423040) == 0:
        poke8(p, 0x423040, 1)
    k = p.read32(mt + 0x9c4)
    if k < last:
        flag, close = p.read8(0x423040), p.read8(gm + 0x344)
        print(json.dumps({
            'flag': flag, 'closeup_at': close, 'server': p.read32(0x42304c), 'receiver': p.read32(0x423054), 'phase': p.read8(gm + 0x55), 'index': p.read32(mt + 0x9c4), 'words': [p.read32(mt + 4 + 4 * j) for j in range(4)],
            'ctl': [p.read32(0x422fc8 + 4 * i) for i in range(4)], 'setup': [p.read8(ai + 0x11) for ai in ais],
            'row': [p.read8(r + 2) for r in rows], 'formation': [p.read8(pl + 0x13f4) for pl in pls],
            'skip': [p.read8(pl + 0x13f5) for pl in pls], 'closeup': p.read8(gm + 0x344), 'order': [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]}), flush=True)
        n += 1
        for pl in pls:
            poke8(p, pl + 0x13f4, 0x77)
    last = p.read32(mt + 0x9c4)
