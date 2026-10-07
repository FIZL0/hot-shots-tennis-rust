"""Parse context/fixtures/trails_s05.bin (tools/record_trails.py)."""
import struct
P = 0xa8 + 48 * 0x30 + 0x90 + 0x40
SIZE = 0xc + 0x100 + 4 * P
def frames(path='context/fixtures/trails_s05.bin'):
    d = open(path, 'rb').read()
    for i in range(len(d) // SIZE):
        s = d[i * SIZE:(i + 1) * SIZE]
        v, clock = struct.unpack_from('<Ii', s, 0)
        pl = []
        for k in range(4):
            o = 0x10c + k * P
            pl.append(dict(t=s[o:o + 0xa8], pts=s[o + 0xa8:o + 0xa8 + 0x900], mot=s[o + 0x9a8:o + 0xa38], mat=s[o + 0xa38:o + P]))
        yield dict(v=v, clock=clock, fx=s[0xc:0x10c], pl=pl)
