"""Dump the per-packet sub-blocks (face morph targets) of an .MDL: python3 mdl_morph.py file.MDL"""
import struct, sys
d = open(sys.argv[1], 'rb').read()
p = 0
def take(n):
    global p
    b = d[p:p + n]; p = (p + n + 15) & ~15; return b
i32 = lambda b, o: struct.unpack_from('<i', b, o)[0]
def tree():
    h = take(0x40); name = take(i32(h, 0x38)); take(0xc0)
    for _ in range(i32(take(4), 0)): tree()
take(1); tree()
for mat in range(i32(take(4), 0)):
    mh = take(0xc)
    for _ in range(i32(mh, 0)):
        bh = take(0x34)
        for pk in range(i32(bh, 0x1c)):
            ph = take(0x60)
            vif = take(i32(ph, 0x34) << 4); take(i32(ph, 0x3c) + 1); n = i32(ph, 0x38); take(n); take(n << 5)
            if ph[0x56]: take(ph[0x57] << 4)
            ns = struct.unpack_from('<h', ph, 0x54)[0]
            if ns > 0:
                print(f'mat {mat} pk {pk} @{p:#x} subs {ns} ph+0x40..0x60', ph[0x40:0x60].hex(' ', 4), 'u56', ph[0x56], ph[0x57])
            for s in range(max(ns, 0)):
                sh = take(0xc); k = i32(sh, 0)
                body = take(k << 4) if k else b''
                if ns > 0:
                    q = [struct.unpack_from('<4f', body, 16 * j) for j in range(min(k, 3))]
                    print(f'  sub {s} hdr', sh.hex(' ', 4), 'k', k, [tuple(round(x, 4) for x in v) for v in q], 'raw0', body[:16].hex(' ', 4))
            if i32(ph, 0x4c) < 0: take(4)
n = i32(take(4), 0)
print('names', [take(i32(take(4), 0)).split(b'\0')[0] for _ in range(n)])
