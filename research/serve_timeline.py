#!/usr/bin/env python3
"""Print a tools/record_serve.py capture as a per-vsync timeline of the serve (B29): the pad's buttons, the
server's serve mode (+0x3fa4/+0x3fa6), press window (+0x3ec0), swing countdown (+0x3ec4), frames since (+0x3f00),
contact offset (+0x3fa0), x, motion id/time/speed, ball y and vy, path bounces, faults. Only lines where something
but the clock changed are printed unless -a."""
import struct, sys
S = 4 + 0x18 + 0x40 + 0x50 + 0x2c0 + 0x20 + 0xe0 + 8 + 0x10
def rows(path):
    d = open(path, "rb").read()
    for k in range(len(d) // S):
        b = d[k * S:(k + 1) * S]
        v = struct.unpack_from("<I", b)[0]
        pad, g, rally, pl = 4, 0x1c, 0x5c, 0xac
        mo, ball, path_, gm = pl + 0x2c0 - 0x20, pl + 0x2c0 + 0x20 - 0x60, pl + 0x2c0 + 0x20 + 0xe0, pl + 0x2c0 + 0x20 + 0xe0 + 8 - 0x50
        u8 = lambda o: b[pl + o - 0x3d40]
        i32 = lambda o: struct.unpack_from("<i", b, pl + o - 0x3d40)[0]
        f = lambda o: struct.unpack_from("<f", b, pl + o - 0x3d40)[0]
        btn = struct.unpack_from("<H", b, pad)[0]
        yield dict(v=v, btn=btn, lx=b[pad + 4], ly=b[pad + 5], mode=(u8(0x3fa4), u8(0x3fa6)), win=u8(0x3ec0), cd=i32(0x3ec4),
                   since=i32(0x3f00), off=i32(0x3fa0), x=f(0x3d70), motion=struct.unpack_from("<i", b, mo + 0x20)[0],
                   t=struct.unpack_from("<f", b, mo + 0x38)[0], sp=struct.unpack_from("<f", b, mo + 0x34)[0],
                   by=struct.unpack_from("<f", b, ball + 0xe0 + 4)[0], vy=struct.unpack_from("<f", b, ball + 0x134)[0],
                   bounces=struct.unpack_from("<i", b, path_ + 4)[0], phase=b[gm + 0x55], raw=b)
if __name__ == "__main__":
    allr = "-a" in sys.argv
    prev = None
    for r in rows([a for a in sys.argv[1:] if a != "-a"][0]):
        key = {k: r[k] for k in ("phase", "btn", "mode", "win", "cd", "off", "motion", "bounces")}
        if allr or key != prev:
            print(f"{r['v']} ph {r['phase']} btn {r['btn']:04x} st {r['lx']:02x},{r['ly']:02x} mode {r['mode']} win {r['win']} cd {r['cd']} since {r['since']} off {r['off']} x {r['x']:.4f} motion {r['motion']:#x} t {r['t']:.2f} sp {r['sp']:.3f} ball y {r['by']:.7f} vy {r['vy']:.7f} b {r['bounces']}")
        prev = key
