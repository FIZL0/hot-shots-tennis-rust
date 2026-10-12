#!/usr/bin/env python3
"""P14c3f: for each trigger creature with a model in a save state's RAM, the draw mode each sub-mesh packet gets in the
model draw (bones' runtime flags & 0x500 → 1, the model's projection flag (+0x130) and the batch's VU1 flag 4 → 2;
only packets with a non-zero mode carry the mesh weight × material factor to VU1).
Usage: p14c3f_weight_modes.py <state.p2s or slot>"""
import os, struct, subprocess, sys, zipfile
src = sys.argv[1]
if src.isdigit():
    common = subprocess.check_output(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"], text=True).strip()
    src = f"{os.path.dirname(common)}-slots/pcsx2/s{os.environ['HST_PCSX2']}/PCSX2/sstates/SCUS-97610 (72326E67).{int(src):02d}.p2s"
ram = zipfile.ZipFile(src).read("eeMemory.bin")
u32 = lambda a: struct.unpack_from("<I", ram, a)[0]
u16 = lambda a: struct.unpack_from("<H", ram, a)[0]
f = lambda a: struct.unpack_from("<f", ram, a)[0]
for o in range(0x100000, len(ram), 4):
    if u32(o) != 0x1d2180 or not u32(o + 0x5c):
        continue
    inst = u32(u32(u32(o + 0x5c)))
    data = u32(inst + 0xc)
    bones = u32(data + 0x64)
    modes, factors = {}, set()
    for m in range(u32(data + 0x68)):
        rec = u32(data + 0x6c) + 16 * m
        n, pk = u32(rec), u32(rec + 4)
        factors.add(round(f(rec + 12), 3))
        for k in range(n):
            p = pk + 0x60 * k
            if ram[p + 0x4c]:
                modes["skip"] = modes.get("skip", 0) + 1
                continue
            batch = u32(u32(p + 0x44) + 4)
            if u16(batch + 0x2c):
                modes["skip"] = modes.get("skip", 0) + 1
                continue
            fl = 0
            for i in range(u32(batch)):
                fl |= u16(bones + u32(batch + 4 + 4 * i) * 0x120 + 0x110)
            mode = (fl & 0x500 != 0) | (2 if ram[inst + 0x130] and ram[batch + 0x30] & 4 else 0)
            modes[mode] = modes.get(mode, 0) + 1
    print(hex(o), "type", ram[o + 0x50], "weight", f(data + 0x5c), "inst+0x130/131", ram[inst + 0x130], ram[inst + 0x131],
          "material factors", sorted(factors), "packet modes", modes)
