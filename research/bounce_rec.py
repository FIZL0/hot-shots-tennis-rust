"""Parser for tools/record_bounce.py captures: header {court, per model (ballbound, chakudan) MOR/MTA entry counts};
per frame vsync, bounce object, fx, ball, 3 contact records, per model {model, mesh, MOR/MTA players, entries}."""
import struct


def frames(path):
    d = open(path, "rb").read()
    court, *counts = struct.unpack_from("<5I", d)
    msz = [0x80 + 0x60 + 0x60 + counts[2 * k] * 0x28 + counts[2 * k + 1] * 0x20 for k in range(2)]
    size = 8 + 0xfc0 + 0x100 + 0x290 + 0xf0 + sum(msz)
    for o in range(20, len(d) - size + 1, size):
        s = d[o:o + size]
        a = 8
        bo, fx, ball, rec = s[a:a + 0xfc0], s[a + 0xfc0:a + 0x10c0], s[a + 0x10c0:a + 0x1350], s[a + 0x1350:a + 0x1440]
        m0 = s[a + 0x1440:a + 0x1440 + msz[0]]
        m1 = s[a + 0x1440 + msz[0]:]
        yield dict(v=struct.unpack_from("<I", s)[0], court=court, counts=counts, bo=bo, fx=fx, ball=ball, rec=rec, models=(m0, m1))


f32 = lambda b, o: struct.unpack_from("<f", b, o)[0]
i32 = lambda b, o: struct.unpack_from("<i", b, o)[0]
