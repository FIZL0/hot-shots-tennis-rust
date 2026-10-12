#!/usr/bin/env python3
"""Reads research/b85_net_rec.py captures: per frame the ball's contact counters, last material and the pop-up
object's words (entries whose texture is a bounce word, 13..27). Usage: b85_parse.py <rec.bin>."""
import struct, sys

SIZE = 4 + 0x290 + 0xf0 + 0x720 + 0x40 + 0x40 + 8 + 0x60


def frames(path):
    d = open(path, "rb").read()
    for o in range(0, len(d) - SIZE + 1, SIZE):
        s = d[o:o + SIZE]
        ball, rec, mgr = s[4:0x294], s[0x294:0x384], s[0x384:0x384 + 0x720]
        cam, gm = s[0x384 + 0x720:0x384 + 0x720 + 0x88], s[0x384 + 0x720 + 0x88:]
        yield dict(v=struct.unpack_from("<I", s)[0], ball=ball, rec=rec, mgr=mgr, cam=cam, gm=gm)


I = lambda b, o: struct.unpack_from("<i", b, o)[0]
F = lambda b, o, n=3: struct.unpack_from(f"<{n}f", b, o)


def words(mgr):
    """(slot, texture, balloon row, pos, timer, flag) of each live entry: 0x30 each from +0x150 (mgr starts at
    +0x130), count at +0x6f0; pos +0, timer +0x10, flag +0x14, texture +0x1c, row +0x29."""
    out = []
    for k in range(I(mgr, 0x6f0 - 0x130)):
        e = mgr[0x20 + k * 0x30:0x20 + (k + 1) * 0x30]
        out.append((k, e[0x1c], e[0x29], tuple(round(x, 4) for x in F(e, 0)), I(e, 0x10), e[0x14]))
    return out


if __name__ == "__main__":
    prev = None
    for f in frames(sys.argv[1]):
        b, m = f["ball"], f["mgr"]
        st = (f["gm"][0x55], I(b, 0x224), I(b, 0x228), I(b, 0x22c), b[0x220], I(m, 0x6f0 - 0x130), I(m, 0x720 - 0x130), I(m, 0x840 - 0x130), I(m, 0x844 - 0x130), m[0x849 - 0x130])
        w = words(m)
        if st != prev or len(sys.argv) > 2:
            print(f["v"], "phase", st[0], "bounces/contacts/special", st[1:4], "mat", st[4], "count", st[5], "last", st[6], "cool", st[7], "made", st[8], "soft", st[9], "pos", ["%.3f" % x for x in F(b, 0xe0)])
            for x in w:
                print("    ", x)
        prev = st
