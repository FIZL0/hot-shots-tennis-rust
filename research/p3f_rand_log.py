#!/usr/bin/env python3
"""P3f: log every rand() call (0x11f840) with its call site, from a save-state load (or live, slot 0).
Usage: p3f_rand_log.py <slot> <frames> <out.bin>. Run under tools/pcsx2.sh. HST_V0: the state's vsync window start
(p3e5_draw_log.py's wait for the late load).
Record (6 × u32): vsync, ra, s0, the match state word (gm+0x54), the rand() state's low word before the call, the match
frame (gm+0x58).
Built like research/p3e5_draw_log.py (entry patched while paused, a few frames after the load)."""
import struct, sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine

VSYNC, GM_PTR, RAND, LIBC = 0x1d5780, 0x422f80, 0x11f840, 0x1b80f0
CODE, DATA, BUF, END = 0x1e02000, 0x1e00000, 0x1e30000, 0x1f80000
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


def build(a, b):
    """The entry stub: logs the call, runs rand()'s first two instructions (a, b) and jumps back."""
    D = DATA & 0xffff
    # 0 lui t4; 1-2 li t5 gm ptr; 3 lw t5 (gm); 4 lw t6 ptr; 5-6 li t7 END; 7 sltu; 8 beq -> skip; 9 nop; 10 lui t7;
    # 11 lw vsync; 12 sw; 13 sw ra; 14 sw s0; 15 lw +0x54; 16 sw; 17-18 li t7 libc; 19 lw t7 (ptr); 20 lw t7 +0xa8; 21 sw;
    # 22 lw +0x58; 23 sw; 24 nop; 25 addiu; 26 sw ptr; 27 skip: a; 28 b; 29 j; 30 nop
    return [lui("t4", DATA >> 16), *li("t5", GM_PTR), lw("t5", 0, "t5"), lw("t6", D, "t4"), *li("t7", END), sltu("t7", "t6", "t7"), i(4, "t7", "zero", 27 - 9), 0,
            lui("t7", VSYNC >> 16), lw("t7", VSYNC & 0xffff, "t7"), sw("t7", 0, "t6"), sw("ra", 4, "t6"), sw("s0", 8, "t6"),
            lw("t7", 0x54, "t5"), sw("t7", 12, "t6"), *li("t7", LIBC), lw("t7", 0, "t7"), lw("t7", 0xa8, "t7"), sw("t7", 16, "t6"),
            lw("t7", 0x58, "t5"), sw("t7", 20, "t6"), 0, addiu("t6", "t6", 24), sw("t6", D, "t4"),
            a, b, j(RAND + 8), 0]


if __name__ == "__main__":
    slot, want, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
    p = Pine()
    if slot:
        p.load_state(slot)
        V0 = int(os.environ.get("HST_V0", 7500))
        while not V0 <= p.read32(VSYNC) < V0 + 50: time.sleep(0.01)
    v0 = p.read32(VSYNC)
    while p.read32(VSYNC) - v0 < 4: time.sleep(0.01)
    a, b = p.read32(RAND), p.read32(RAND + 4)
    stub = build(a, b)
    p.pause()
    p.write32(DATA, BUF)
    for n, w in enumerate(stub): p.write32(CODE + 4 * n, w)
    p.write32(RAND, j(CODE))
    p.write32(RAND + 4, 0)
    p.resume()
    v0 = p.read32(VSYNC)
    try:
        while p.read32(VSYNC) - v0 < want and p.read32(DATA) < END - 0x1000:
            time.sleep(0.1)
    finally:
        p.pause()
        p.write32(RAND + 4, b)
        p.write32(RAND, a)
        end = p.read32(DATA)
        p.resume()
    data = b"".join(p.read_block(x, min(0x10000, end - x)) for x in range(BUF, end, 0x10000)) if end > BUF else b""
    open(out, "wb").write(data)
    print(f"{len(data) // 24} calls over vsync {v0}..{p.read32(VSYNC)}")
