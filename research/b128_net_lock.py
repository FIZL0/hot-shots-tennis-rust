#!/usr/bin/env python3
"""B128: does a swing locked before a net touch still strike the ball? Loads a bot game, waits for the
<n>-th time a player's contact search locks (countdown +0x3ec4 >= <min>), then puts the ball on the far side
of the net just short of the cord, flying at it (z speed <vz> m/frame toward the locked player, height <y>
m), and logs every frame until <frames> have passed: match phase gm+0x55, rally shot count 0x423060,
last hitter 0x423058, ball pos/vel and counters (+0x224 bounces, +0x228 contacts, +0x22c special touches),
the predictor's +0x224/+0x228, and each player's branch +0x3ec1, countdown +0x3ec4 and aim-ready +0x3e48.
Usage: b128_net_lock.py <slot> <n> <min> <x> <y> <vz> <frames> > log."""
import struct, sys, time
sys.path.insert(0, "tools")
from pine import Pine

VSYNC, GM_PTR, SHOTS, HITTER = 0x1d5780, 0x422f80, 0x423060, 0x423058
slot, nth, least, x, y, vz, want = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]), *map(float, sys.argv[4:7]), int(sys.argv[7])
f2u = lambda f: struct.unpack("<I", struct.pack("<f", f))[0]
s32 = lambda u: struct.unpack("<i", struct.pack("<I", u))[0]
p = Pine()
p.load_state(slot)
time.sleep(2)  # the load lands asynchronously; pointers read before it are stale
gm = p.read32(GM_PTR)
ball, pred = p.read32(gm + 0x88), p.read32(gm + 0x98)
players = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
F = lambda b, o, n=3: struct.unpack_from(f"<{n}f", b, o)
I = lambda b, o: struct.unpack_from("<i", b, o)[0]
last, locks, prev, poked, n = p.read32(VSYNC), 0, [-1] * 4, None, 0
while n < want:
    v = p.next_frame(last)
    last = v
    b = p.read_block(ball, 0x230)
    pl = [(p.read8(a + 0x3ec1), s32(p.read32(a + 0x3ec4)), p.read8(a + 0x3e48), F(p.read_block(a + 0x3d70, 0x10), 0)) for a in players]
    line = (f"{v} ph {p.read8(gm + 0x55)} shots {s32(p.read32(SHOTS))} hitter {s32(p.read32(HITTER))} "
            f"ball {['%.3f' % c for c in F(b, 0xe0)]} vel {['%.3f' % c for c in F(b, 0x130)]} "
            f"n {I(b, 0x224)}/{I(b, 0x228)}/{I(b, 0x22c)} pred {s32(p.read32(pred + 0x224))}/{s32(p.read32(pred + 0x228))} "
            + " ".join(f"P{i}[{br} {cd} {aim}]" for i, (br, cd, aim, _) in enumerate(pl)))
    if poked is None and p.read8(gm + 0x55) == 3:
        for i, (br, cd, _, pos) in enumerate(pl):
            if br in (1, 2, 4) and least <= cd < 40 and not least <= prev[i] < 40:
                locks += 1
                if locks == nth:
                    side = 1.0 if pos[2] > 0 else -1.0  # the locked player's end; the ball goes on the other
                    for k, c in enumerate([x, -y, -side * 0.3]):
                        p.write32(ball + 0xe0 + 4 * k, f2u(c))
                    for k, c in enumerate([0.0, 0.0, side * vz]):
                        p.write32(ball + 0x130 + 4 * k, f2u(c))
                    poked = v
                    line += f"  <- P{i} locked {cd} (branch {br}) at z {pos[2]:.2f}: ball put at the net"
        prev = [c[1] for c in pl]
    if poked is not None:
        n += 1
    print(line, flush=True)
print("done", n)
