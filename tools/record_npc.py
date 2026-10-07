#!/usr/bin/env python3
"""Record the walking spectators every frame from a save-state load (bot games: slot 5).
Usage: record_npc.py <slot> <frames> <out.bin>. The walkers are found by vtable in the state's RAM (the .p2s file).
Header: u32 count, then count × u32 walker address. Sample = u32 vsync, 0x422f80 + 0x180 (globals), the shared MT
(*(*(*(gm+0x84)+0x154)+0x50), 0x9d0: index at +0x9c4), the gallery manager *(0x43b1c0) +0x680..+0x900 and
+0x1b60..+0x1b80, the judge *(0x42d6c0) +0x420 (8 bytes), then per walker its object (0x310), its animation
controller *(+0x60) (0x40) and that controller's animation header *(ctrl+0x24) (0x30). Stops when the match ends
(the match object or a walker goes away, or a frame never reads the same twice)."""
import struct, sys, time, zipfile
from pine import Pine

VSYNC, GM_PTR, WALKER = 0x1d5780, 0x422f80, 0x1d1de0
STATES = "/home/ryha/Emulation/saves/ps2/states/SCUS-97610 (72326E67).%02d.p2s"
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
ram = zipfile.ZipFile(STATES % slot).read("eeMemory.bin")
walkers = [a for a in range(0x100000, len(ram), 4) if struct.unpack_from("<I", ram, a)[0] == WALKER]
out.write(struct.pack(f"<I{len(walkers)}I", len(walkers), *walkers))

p = Pine()
p.load_state(slot)
time.sleep(0.3)

def regions():
    gm, mgr = p.read32(GM_PTR), p.read32(0x43b1c0)
    r = [(GM_PTR, 0x180), (p.read32(p.read32(p.read32(gm + 0x84) + 0x154) + 0x50), 0x9d0),
         (mgr + 0x680, 0x280), (mgr + 0x1b60, 0x20), (p.read32(0x42d6c0) + 0x420, 8)]
    for w in walkers:
        c = p.read32(w + 0x60)
        r += [(w, 0x310), (c, 0x40), (p.read32(c + 0x24), 0x30)]
    return r

last, n = p.read32(VSYNC), 0
while n < want:
    v = p.read32(VSYNC)
    if v == last:
        continue
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    time.sleep(0.004)
    gm = p.read32(GM_PTR)
    if any(p.read32(w) != WALKER for w in walkers):
        sys.exit(f"walkers gone at vsync {v} (match over), {n} samples")
    r = regions()
    a = p.read_regions(r)
    for _ in range(50):
        r2 = regions()
        b = p.read_regions(r2)
        if a == b and r == r2 and p.read32(VSYNC) == v: break
        if p.read32(GM_PTR) != gm: sys.exit(f"match object gone at vsync {v} (match over), {n} samples")
        a, r = b, r2
    else:
        sys.exit(f"vsync {v} never read the same twice (match over?), {n} samples")
    out.write(struct.pack("<I", v) + a)
    out.flush()
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
