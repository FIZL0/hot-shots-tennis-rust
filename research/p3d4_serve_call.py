#!/usr/bin/env python3
"""P3d4: the doubles serve call. Loads a save state, puts the server's team at match point (points 3-0, games and
sets one short) unless `asis`, and logs every shared-generator draw like p3e5_draw_log.py (u32 vsync, ra, the
caller's ra *(sp+0x10 in 0x3553d0's frame), s0, match frame) until the server has tossed.
Usage: p3d4_serve_call.py <slot> <frames> <out.bin> [asis | other] (other: the receiving team at match point). Run under tools/pcsx2.sh.
out.bin: b"P3D4", u32 players, server, toss kind (+0x3ea0), points[2], games[2], sets[2], games to win, sets to
win, hud +0x50, +0x54, tiebreak 0x31661a, deuce 0x316620, adv 0x316628, then per player (4) +0x3b96 at the
start and the end, then u32 count and the draws."""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine

VSYNC, GM_PTR, DRAW = 0x1d5780, 0x422f80, 0x19f5c0
CODE, DATA, BUF, END = 0x1e02000, 0x1e00000, 0x1e30000, 0x1f80000
PTS, GAMES, SETS, SERVER, HUD = 0x423064, 0x42306c, 0x423074, 0x42304c, 0x42d6c0
R = dict(zero=0, a0=4, t4=12, t5=13, t6=14, t7=15, s0=16, sp=29, ra=31)


def i(op, rs, rt, imm): return op << 26 | R[rs] << 21 | R[rt] << 16 | imm & 0xffff
def lw(rt, off, rs): return i(0x23, rs, rt, off)
def sw(rt, off, rs): return i(0x2b, rs, rt, off)
def lui(rt, imm): return i(0x0f, "zero", rt, imm)
def ori(rt, rs, imm): return i(0x0d, rs, rt, imm)
def addiu(rt, rs, imm): return i(0x09, rs, rt, imm)
def sltu(rd, rs, rt): return R[rs] << 21 | R[rt] << 16 | R[rd] << 11 | 0x2b
def j(a): return 0x02 << 26 | (a >> 2) & 0x3ffffff
def li(rt, v): return [lui(rt, v >> 16), ori(rt, rt, v & 0xffff)]


slot, want, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
mode = (sys.argv[4:5] or ["mp"])[0]
asis = mode == "asis"
p = Pine()
p.load_state(slot)
V0 = int(os.environ.get("HST_V0", 7500))
while not V0 <= p.read32(VSYNC) < V0 + 50: time.sleep(0.01)
v0 = p.read32(VSYNC)
while p.read32(VSYNC) - v0 < 4: time.sleep(0.01)
gm0 = p.read32(GM_PTR)
MT = p.read32(gm0 + 0x80)
players = p.read32(0x422fa4)
pl = [p.read32(gm0 + 0xa8 + 4 * k) for k in range(players)]
a, b = p.read32(DRAW), p.read32(DRAW + 4)
stub = [lui("t4", DATA >> 16), *li("t5", MT), i(5, "a0", "t5", 25 - 4), 0, lw("t6", DATA & 0xffff, "t4"),
        *li("t7", END), sltu("t7", "t6", "t7"), i(4, "t7", "zero", 25 - 10), 0,
        lui("t7", VSYNC >> 16), lw("t7", VSYNC & 0xffff, "t7"), sw("t7", 0, "t6"), sw("ra", 4, "t6"),
        lw("t7", 0x10, "sp"), sw("t7", 8, "t6"), sw("s0", 12, "t6"),
        *li("t7", GM_PTR), lw("t7", 0, "t7"), lw("t7", 0x58, "t7"), sw("t7", 16, "t6"), addiu("t6", "t6", 20), sw("t6", DATA & 0xffff, "t4"), a, b, j(DRAW + 8), 0]
p.pause()
server = p.read32(SERVER)
if not asis:
    t, hud = server & 1 ^ (mode == "other"), p.read32(HUD)
    gw, sw_ = p.read32(hud + 0x54), p.read32(hud + 0x50)
    for base, mine in ((PTS, 3), (GAMES, gw - 1), (SETS, sw_ - 1)):
        p.write32(base + 4 * t, mine)
        p.write32(base + 4 * (t ^ 1), 0)
called0 = [p.read8(q + 0x3b96) for q in pl]
p.write32(DATA, BUF)
for n, w in enumerate(stub): p.write32(CODE + 4 * n, w)
p.write32(DRAW, j(CODE))
p.write32(DRAW + 4, 0)
p.resume()
v0 = p.read32(VSYNC)
srv = pl[server]
try:
    while p.read32(VSYNC) - v0 < want and p.read32(GM_PTR) == gm0 and p.read32(DATA) < END - 0x1000:
        # the toss: state 2 (+0x3fa6) entered
        if p.read8(srv + 0x3fa6) == 2:
            t1 = p.read32(VSYNC)
            while p.read32(VSYNC) - t1 < 3: time.sleep(0.01)
            break
        time.sleep(0.01)
finally:
    p.pause()
    p.write32(DRAW + 4, b)
    p.write32(DRAW, a)
hud = p.read32(HUD)
head = [players, server, p.read32(srv + 0x3ea0)] + [p.read32(x + 4 * k) for x in (PTS, GAMES, SETS) for k in (0, 1)]
head += [p.read32(0x423044), p.read32(0x423048), p.read32(hud + 0x50), p.read32(hud + 0x54),
         p.read8(0x31661a), p.read8(0x316620), p.read8(0x316628)]
called1 = [p.read8(q + 0x3b96) for q in pl]
p.resume()
end = p.read32(DATA)
data = b"".join(p.read_block(x, 0x10000) for x in range(BUF, end, 0x10000))[: end - BUF]
pad = lambda c: c + [0] * (4 - len(c))
open(out, "wb").write(b"P3D4" + struct.pack(f"<{len(head)}I8I", *head, *pad(called0), *pad(called1))
                      + struct.pack("<I", len(data) // 20) + data)
print("head", head, "called", called0, "->", called1, "draws", len(data) // 20)
for k in range(len(data) // 20):
    v, ra, c, s0, f = struct.unpack_from("<5I", data, 20 * k)
    print(f"  {v} ra {ra:#x} caller {c:#x} s0 {s0:#x}")
