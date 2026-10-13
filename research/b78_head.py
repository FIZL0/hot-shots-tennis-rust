#!/usr/bin/env python3
"""B78: sample the umpire's head look-at from the running game (slot given, default 5) and check our reading of it:
the head node's rest local (its model node's +0x40), its parent's world (via +0x100), the target (actor +0xc0) → the turned local the
game stores (+0x80). Prints the actor's motion, time, over flag and the max error per sample."""
import math, struct, sys, time
sys.path.insert(0, "tools")
from pine import Pine
p = Pine()
slot = int(sys.argv[1]) if len(sys.argv) > 1 else 5
n = int(sys.argv[2]) if len(sys.argv) > 2 else 12
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
mat = lambda a: [[f(a + 16 * r + 4 * c) for c in range(4)] for r in range(4)]
mul = lambda A, B: [[sum(A[i][k] * B[k][j] for k in range(4)) for j in range(4)] for i in range(4)]
def inv(M):
    import numpy
    return numpy.linalg.inv(numpy.array(M)).tolist()
def rx(a): c, s = math.cos(a), math.sin(a); return [[1,0,0,0],[0,c,s,0],[0,-s,c,0],[0,0,0,1]]
def ry(a): c, s = math.cos(a), math.sin(a); return [[c,0,-s,0],[0,1,0,0],[s,0,c,0],[0,0,0,1]]
p.load_state(slot); time.sleep(2)
for k in range(n):
    time.sleep(1.5)
    p.pause()
    act = p.read32(0x43b1c8); pl = p.read32(act + 0x60); node = p.read32(pl + 0x84)
    motion, d7, t = p.read32(act + 0xd0), p.read8(act + 0xd7), f(pl + 0x38)
    tgt = [f(act + 0xc0 + 4 * i) for i in range(4)]
    L, P, R = mat(p.read32(p.read32(node + 0x108) + 0x10) + 0x40), mat(p.read32(node + 0x100)), mat(node + 0x80)
    M = inv(mul(L, P))
    q = [sum(tgt[i] * M[i][j] for i in range(4)) for j in range(4)]
    x, y, z = -q[0], q[1], -q[2]
    yaw = max(-1.2217305, min(1.2217305, 0.7 * math.atan2(y, z)))
    tilt = max(-0.43633232, min(0.61086524, 0.7 * math.atan2(x, math.hypot(y, z))))
    E = mul(ry(tilt), mul(rx(yaw), L))
    err = max(abs(E[i][j] - R[i][j]) for i in range(4) for j in range(4))
    print(f"motion {motion} over {d7} t {t:.1f} target {[round(v, 2) for v in tgt[:3]]} yaw {math.degrees(yaw):.1f} tilt {math.degrees(tilt):.1f} err {err:.2e}")
    p.resume()
