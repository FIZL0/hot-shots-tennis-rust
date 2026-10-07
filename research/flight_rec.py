"""Parser for tools/record_flight.py captures: per frame vsync, fx, flight ribbon, its samples, ball glow, ball,
court marker 8."""
import struct

SIZE = 8 + 0x100 + 0xb0 + 0x320 + 0x70 + 0x290 + 0x40


def frames(path):
    d = open(path, "rb").read()
    for o in range(0, len(d) - SIZE + 1, SIZE):
        s = d[o:o + SIZE]
        a = 8
        fx, fl, pts, glow, ball, mark = (s[a:a + 0x100], s[a + 0x100:a + 0x1b0], s[a + 0x1b0:a + 0x4d0],
                                         s[a + 0x4d0:a + 0x540], s[a + 0x540:a + 0x7d0], s[a + 0x7d0:a + 0x810])
        yield dict(v=struct.unpack_from("<I", s)[0], fx=fx, fl=fl, pts=pts, glow=glow, ball=ball, mark=mark)
