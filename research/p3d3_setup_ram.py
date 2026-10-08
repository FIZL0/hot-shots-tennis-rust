#!/usr/bin/env python3
"""P3d3: what a match's setup leaves in RAM from the shared generator's draws, read offline from save states.
Usage: p3d3_setup_ram.py out.bin state.p2s...
Per state (little-endian): u32 court, players, games, sets, match seed (0x42303c); u64 rand() state; per player
(4) u32 voice-bank b flag (+0x17d8), has an AI object (+0x80 ≠ 0); u32 gallery pick (0x3125a8), its 4 used flags
(sound manager +0x1d0); the AI generator (0x427130, 0x9d0 bytes); the weather schedule (gm+0xd8, 65 bytes, pad 3)
and its wind (gm+0x11c, 65 × (f32 speed, i32 degrees))."""
import struct, sys, zipfile

out = open(sys.argv[1], "wb")
for f in sys.argv[2:]:
    z = zipfile.ZipFile(f)
    m = z.read(next(n for n in z.namelist() if "eeMemory" in n))
    r = lambda a: struct.unpack_from("<I", m, a)[0]
    gm = r(0x422f80)
    assert 0 < gm < 0x2000000, f"{f}: not in a match"
    rec = struct.pack("<5I", r(0x422f90), r(0x422fa4), r(0x423044), r(0x423048), r(0x42303c))
    rec += m[r(0x1b80f0) + 0xa8:][:8]
    for i in range(4):
        p = r(gm + 0xa8 + 4 * i) if i < r(0x422fa4) else 0
        rec += struct.pack("<2I", r(p + 0x17d8) if p else 0, int(bool(p and r(p + 0x80))))
    rec += struct.pack("<I", r(0x3125a8)) + m[r(0x312540) + 0x1d0:][:4]
    rec += m[0x427130:][:0x9d0] + m[gm + 0xd8:][:65] + bytes(3) + m[gm + 0x11c:][:65 * 8]
    out.write(rec)
    print(f, "court", r(0x422f90), "players", r(0x422fa4), "pick", r(0x3125a8))
