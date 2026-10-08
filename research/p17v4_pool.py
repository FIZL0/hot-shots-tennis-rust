#!/usr/bin/env python3
"""Is a GS dump's shadow-caster texture pool the save state's own? (P17v4)
Usage: p17v4_pool.py <state.p2s> dump.gs ...
The build's receiver pass samples 16 PSMT4 128×128 textures at TBP 0x3308 + 0x20·i, which the game never rewrites
while it plays. So a rebuild matches its save state only if that pool, as left by the load, equals the copy in the
state's GS.bin. Prints, for each dump, whether it does."""
import sys, zipfile
sys.path.insert(0, "research")
from p17v_gssim import vram

POOL = slice(0x3308 * 256, 0x3508 * 256)  # 16 textures × 0x20 blocks × 256 bytes
gs = zipfile.ZipFile(sys.argv[1]).read("GS.bin")  # zstd member: needs Python ≥ 3.14
for d in sys.argv[2:]:
    print(d, "pool = state's" if vram(d).tobytes()[POOL] in gs else "pool differs from the state")
