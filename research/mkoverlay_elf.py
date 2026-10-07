# Build one ELF per Metrowerks overlay: main ELF segment 0 + overlay file loaded at its address (+bss).
import struct, sys
elf = bytearray(open("context/iso/SCUS_976.10","rb").read())
phoff, = struct.unpack_from("<I", elf, 0x1c); phnum, = struct.unpack_from("<H", elf, 0x2c)
for seg, name in [(2,"GAME"),(3,"MENU"),(4,"MOVIE")]:
    out = bytearray(elf); ov = open(f"context/iso/ZZBIN/{name}.BIN","rb").read()
    load, = struct.unpack_from("<I", ov, 8); bss, = struct.unpack_from("<I", ov, 0x14)
    off = len(out); out += ov
    for i in range(1, phnum):
        p = phoff + i*32
        if i == seg: struct.pack_into("<IIIIII", out, p, 1, off, load, load, len(ov), len(ov)+bss)
        else: struct.pack_into("<I", out, p, 0)  # PT_NULL
    struct.pack_into("<IHH", out, 0x20, 0, 0, 0)  # drop section headers (would conflict)
    struct.pack_into("<HHH", out, 0x2e, 0, 0, 0)
    open(f"context/elf/hst_{name.lower()}.elf","wb").write(out)
    print(name, hex(load), hex(len(ov)), hex(bss))
