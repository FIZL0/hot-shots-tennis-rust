#!/usr/bin/env python3
"""Export the static shadow-texture pass for `shade::pass_texture`'s test (P17v2).

Usage: p17v2_fixture.py load.gs cap.gs out.txt [frame]   (load.gs from p17v2_load.py, cap.gs holding the finished
textures at 0x3308 + 0x20·i, e.g. context/p17v/cap0.gs; frame: the vsync whose pass is exported, default 396)

Walks the pass as p17v2_replay.py does and writes, per texture: `T`; per draw (a batch with one GS state) `D aref
modulate` plus, when textured, `log2w log2h wrapS wrapT alpha-hex` (wrap `r` repeat or `c<min>,<max>`); per
triangle `V` and three × (x y in the 256² target, 12.4; S T as f32 bits; vertex alpha); then `E` and the 128²
texture from cap.gs as one hex digit per texel."""
import sys
import numpy as np
sys.path.insert(0, "research")
sys.path.insert(0, "research/tools")
from gsdump import packets
from p17v_gssim import vram, addr4
import p17v2_replay as R

bits = lambda f: int(np.float32(f).view(np.uint32))


class Gs(R.Gs):
    def __init__(self, vm, frame, out):
        super().__init__(vm, frame)
        self.out, self.key, self.n = out, None, -1

    def prim(self, prim, vs):
        g = self.r.get
        if prim & 7 == 4:
            assert g(0x18) == 0x780000007800 and g(0x40) == 0xfb000400fb0004 and g(0x47) & 0xc001 == 0x4001
            n = len([b for b in self.blits if b >= 0x3308])
            if n != self.n:
                self.out.append("T")
                self.n, self.key = n, None
            key = (prim & ~8, g(0x06), g(0x14), g(0x08), g(0x47))
            if key != self.key:
                self.key = key
                d = f"D {(g(0x47) >> 4) & 0xff} {int(prim & 0x10 != 0 and (g(0x06) >> 35) & 3 == 0)}"
                if prim & 0x10:
                    tex0, clamp = g(0x06), g(0x08)
                    tw, th = (tex0 >> 26) & 15, (tex0 >> 30) & 15
                    wrap = []
                    for wm, mn, mx, sz in ((clamp & 3, (clamp >> 4) & 0x3ff, (clamp >> 14) & 0x3ff, 1 << tw),
                                           ((clamp >> 2) & 3, (clamp >> 24) & 0x3ff, (clamp >> 34) & 0x3ff, 1 << th)):
                        assert wm in (0, 1, 2), wm
                        wrap.append("r" if wm == 0 else f"c0,{sz - 1}" if wm == 1 else f"c{min(mn, sz - 1)},{min(mx, sz - 1)}")
                    alpha = ((self.texture(tex0) >> 24) & 0xff).astype(np.uint8)
                    d += f" {tw} {th} {wrap[0]} {wrap[1]} {alpha.tobytes().hex()}"
                self.out.append(d)
            self.out.append("V " + " ".join(f"{x - 0x7800} {y - 0x7800} {bits(s):08x} {bits(t):08x} {(c >> 24) & 0xff}"
                                            for x, y, s, t, q, c, *_ in vs))
        super().prim(prim, vs)


def main():
    dump, cap, path = sys.argv[1:4]
    frame = int(sys.argv[4]) if len(sys.argv) > 4 else 396
    out = [f"# p17v2_fixture.py {dump} {cap} {frame}"]
    gs = Gs(vram(dump), frame, out)
    for p in packets(open(dump, "rb").read()):
        if p[0] == "vsync":
            gs.frame += 1
            if gs.frame > frame:
                break
        else:
            gs.gif(p[2])
            if len([b for b in gs.blits if b >= 0x3308]) >= 16:
                break
    vm = vram(cap)
    texs = []
    for i in range(16):
        nib = [(vm[a >> 1] >> (4 * (a & 1))) & 15 for a in (addr4(x, y, 0x3308 + 0x20 * i, 2) for y in range(128) for x in range(128))]
        texs.append("E " + "".join(f"{v:x}" for v in nib))
    # each texture's draws end where the next `T` starts; its expected texels follow them
    res, k = [], 0
    for line in out:
        if line == "T" and k:
            res.append(texs[k - 1])
        k += line == "T"
        res.append(line)
    res.append(texs[k - 1])
    assert k == 16, k
    open(path, "w").write("\n".join(res) + "\n")
    print(path, len(res), "lines")


if __name__ == "__main__":
    main()
