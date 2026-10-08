#!/usr/bin/env python3
"""P17k: find which newlib rand() output seeded the shared MT19937 the weather schedule was drawn from.
Reads (under tools/pcsx2.sh, a match loaded) rand's 64-bit state, the match seed (0x42303c) and the schedule, walks the
LCG back and tries each earlier output as the MT seed (init_genrand) for the schedule draw. Prints the hit."""
import struct, sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
A, M = 0x5851f42d4c957f2d, (1 << 64) - 1
AINV = pow(A, -1, 1 << 64)
p = Pine()
if len(sys.argv) > 1:
    import time; p.load_state(int(sys.argv[1])); time.sleep(2)
gm = p.read32(0x422f80)
state = p.read64(p.read32(0x1b80f0) + 0xa8)
seed = p.read32(0x42303c)
weather = p.read_block(gm + 0xd8, 72)[:65]
wind = p.read_block(gm + 0x11c, 65 * 8)
court, players = p.read32(0x422f90), p.read32(0x422fa4)
games, sets = p.read32(0x423044), p.read32(0x423048)

class Mt:
    def __init__(s, seed):
        s.m = [seed & 0xffffffff]
        for i in range(1, 624): s.m.append((i + (s.m[-1] ^ (s.m[-1] >> 30)) * 0x6c078965) & 0xffffffff)
        s.i = 624
    def __call__(s):
        if s.i >= 624:
            m = s.m
            for k in range(624):
                y = (m[k] & 0x80000000) | (m[(k + 1) % 624] & 0x7fffffff)
                m[k] = m[(k + 397) % 624] ^ (y >> 1) ^ (0x9908b0df if y & 1 else 0)
            s.i = 0
        y = s.m[s.i]; s.i += 1
        y ^= y >> 11; y ^= (y << 7) & 0x9d2c5680; y ^= (y << 15) & 0xefc60000
        return y ^ (y >> 18)

def weathers(mt, odds):
    r = lambda: (mt() >> 16) & 0x7fff
    out = [0] * 65
    if players > 1:
        i = 0
        while i < 65:
            if r() % 100 < odds[0]:
                n = odds[1] + r() % (odds[2] - odds[1] + 1)
                k = 0
                while k < n and i + k < 65: out[i + k] = 1; k += 1
                i += k
            i += 1
        # only the cloudy pass is checked here; the rest is the Rust test's job
    return out

# the court's weather odds row straight from RAM (the exe table)
odds = struct.unpack("<9i", p.read_block(0x41ce00 + (court - 1) * 0x24, 40)[:36])
want = list(weather)
s, outs = state, []
for n in range(200000):
    outs.append(((s >> 32) & 0x7fffffff, s))
    s = ((s - 1) * AINV) & M
idx = [k for k, (o, _) in enumerate(outs) if o == seed]
print("court", court, "players", players, "games", games, "sets", sets, "odds", odds, "match seed", hex(seed), "found at back", idx[:3])
start = idx[0] if idx else 0
for k in range(start, len(outs)):
    o, st = outs[k]
    got = weathers(Mt(o), odds)
    if all((g == 1) == (w == 1) for g, w in zip(got, want) if w in (0, 1)):
        print("schedule seed", hex(o), "rand outputs back", k, "after match seed", k - start, "state", hex(st))
        break
else:
    print("not found")
