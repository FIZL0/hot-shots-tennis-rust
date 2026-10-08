"""P17k: the characters' noise deformers (vtable 0x1d0cf8) in a running match: per frame, each one's phase and
rate, and the wind rate. Usage: tools/pcsx2.sh python3 research/p17k_noise_rec.py <slot> <frames> > out.txt"""
import struct, sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine, VSYNC

p = Pine()
p.load_state(int(sys.argv[1]))
ram = b"".join(p.read_block(a, 0x10000) for a in range(0x100000, 0x2000000, 0x10000))
objs = [0x100000 + o for o in range(0, len(ram), 4) if struct.unpack_from("<I", ram, o)[0] == 0x1d0cf8]
for o in objs:
    hdr = struct.unpack_from("<I", ram, o - 0x100000 + 0x14)[0]
    e = ram[hdr - 0x100000:hdr - 0x100000 + 0x30]
    print(f"# obj {o:#x} entry {hdr:#x} {struct.unpack_from('<HH', e)} {struct.unpack_from('<4f', e, 8)} {e[0x20:0x30].split(bytes(1))[0]}")
v = p.read32(VSYNC)
for _ in range(int(sys.argv[2])):
    v = p.next_frame(v)
    regions = [(o + 0x18, 0x40) for o in objs] + [(0x1bb5a8, 8)]
    b = p.settle(regions, v)
    if b is None: continue
    row = [f"{struct.unpack_from('<I', b, len(objs) * 0x40)[0]:08x}"]
    for i in range(len(objs)):
        q = b[i * 0x40:(i + 1) * 0x40]  # from +0x18
        row.append(" ".join(f"{struct.unpack_from('<I', q, k)[0]:08x}" for k in (0x4, 0x8, 0x18, 0x1c, 0x28)))
    print(v, " | ".join(row), flush=True)
