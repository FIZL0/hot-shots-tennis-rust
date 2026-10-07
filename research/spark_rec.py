"""Parse context/fixtures/sparks_s05.bin (tools/record_sparks.py)."""
import struct
N = 25
SIZE = 4 + 0x100 + 0x70 + N * 0x40 + N * 0x50 + 0x290 + 0x40
def frames(path="context/fixtures/sparks_s05.bin"):
    d = open(path, "rb").read()
    for i in range(len(d) // SIZE):
        s = d[i * SIZE:(i + 1) * SIZE]
        o = 4; fx = s[o:o + 0x100]; o += 0x100; sp = s[o:o + 0x70]; o += 0x70
        parts = [s[o + j * 0x40:o + (j + 1) * 0x40] for j in range(N)]; o += N * 0x40
        table = [s[o + j * 0x50:o + (j + 1) * 0x50] for j in range(N)]; o += N * 0x50
        ball = s[o:o + 0x290]; o += 0x290; mark = s[o:o + 0x40]
        yield dict(v=struct.unpack_from("<I", s)[0], fx=fx, sp=sp, parts=parts, table=table, ball=ball, mark=mark)
