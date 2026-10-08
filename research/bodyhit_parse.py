"""Parser for research/bodyhit_rec.py captures."""
import struct
PL = 0x80 + 0x40 + 0x80
SIZE = 4 + 0x60 + 0x290 + 4 * PL

def frames(path):
    d = open(path, "rb").read()
    for o in range(0, len(d) - SIZE + 1, SIZE):
        s = d[o:o + SIZE]
        gm, ball = s[4:0x64], s[0x64:0x64 + 0x290]
        pls = []
        for i in range(4):
            b = s[0x64 + 0x290 + i * PL:][:PL]
            pls.append(dict(hit_tick=struct.unpack_from("<i", b, 0x20)[0], mode=b[0x80 + 0x25], anim=b[0xc0:]))
        f = lambda off, n=3: struct.unpack_from("<%df" % n, ball, off)
        yield dict(v=struct.unpack_from("<I", s)[0], phase=gm[0x55], tick=struct.unpack_from("<i", gm, 0x58)[0],
                   pos=f(0xe0), vel=f(0x130), disp=f(0x140), bounces=struct.unpack_from("<i", ball, 0x224)[0],
                   ball=ball, players=pls)

if __name__ == "__main__":
    import sys
    for fr in frames(sys.argv[1]):
        print(fr["v"], fr["phase"], fr["tick"], fr["bounces"], ["%.4f" % x for x in fr["pos"]], ["%.4f" % x for x in fr["vel"]],
              [(p["mode"], p["hit_tick"]) for p in fr["players"]])
