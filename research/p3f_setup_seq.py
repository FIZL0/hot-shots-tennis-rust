#!/usr/bin/env python3
"""P3f: a research/p3f_menu_log.py log's rand() calls from the match seed to the first point, by caller: the seed, the
lens flare's construction (48), the sound reseeds, the weather particles (5 each), the flare's ticks (24 each), the new
point's two seeds. Usage: p3f_setup_seq.py [--fixture out.bin] log.bin...
--fixture (crates/hst-sim/tests/rng.rs `setup_rand_like_the_game`): per log 3 × u32, the rand() state's low word
before the seed, before the effects seed (after the clouds) and before the first point's shared seed."""
import struct, sys, collections

FLARE, SEED, POINT, EFFECTS = 0x137f20, 0x322f1c, 0x324ca0, 0x1a3154
args = sys.argv[1:]
fix = None
if args[0] == "--fixture": fix, args = open(args[1], "wb"), args[2:]
for f in args:
    d = open(f, "rb").read()
    r = [struct.unpack_from("<6I", d, i) for i in range(0, len(d), 24)]
    s = next(i for i, x in enumerate(r) if x[1] == SEED)
    e = next(i for i, x in enumerate(r) if x[1] == POINT)
    if fix: fix.write(struct.pack("<3I", r[s][4], next(x[4] for x in r if x[1] == EFFECTS), r[e][4]))
    seq = []
    for x in r[s:e]:
        k = "flare tick" if x[1] == FLARE else "flare made" if 0x135100 < x[1] < 0x1351a0 else \
            "particles" if 0x32c500 < x[1] < 0x32c700 else hex(x[1])
        if seq and seq[-1][0] == k: seq[-1][1] += 1
        else: seq.append([k, 1, x[0]])
    print(f, f"menu calls before the seed {s}, seed..point {e - s}")
    for k, n, v in seq: print(f"  {k:12} {n:6}  vsync {v}")
