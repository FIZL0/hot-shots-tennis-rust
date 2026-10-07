#!/usr/bin/env python3
"""Dump the per-character TParam records (14 × 0x118 bytes, as the parser left them) and each player's copy of its
record (+0x12d8, 0x118 bytes, with +0x12bc character) from a save state. Under tools/pcsx2.sh:
p7c_dump.py <slot> <out.bin>  → out.bin: table, then per player u32 character + 0x118-byte copy."""
import struct, sys, time
sys.path.insert(0, __import__('os').path.join(__import__('os').path.dirname(__file__), '..', 'tools'))
from pine import Pine

slot, out = int(sys.argv[1]), sys.argv[2]
p = Pine()
p.load_state(slot)
time.sleep(2)
table = p.read_block(0x2f0880, 14 * 0x118)
gm = p.read32(0x422f80)
blob = bytearray(table)
for i in range(4):
    pl = p.read32(gm + 0xa8 + 4 * i)
    if not pl: break
    blob += struct.pack('<I', p.read32(pl + 0x12bc)) + p.read_block(pl + 0x12d8, 0x118)
    print('player', i, 'char', p.read32(pl + 0x12bc))
open(out, 'wb').write(blob)
print(len(blob), 'bytes')
