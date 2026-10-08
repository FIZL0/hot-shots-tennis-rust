#!/usr/bin/env python3
"""P3e5: log every draw on the AI's generator (0x427130) with its call site, from a save-state load (slot 5).
Usage: p3e5_draw_log.py <slot> <frames> <out.bin> [shared [stack]]. With `shared`, logs the shared generator *(gm+0x80) instead, and the word at sp+stack (hex, default 0x10) in place of sp+0x10 (P3d). Run under tools/pcsx2.sh.
A hook on the MT draw (0x19f5c0) appends (u32 vsync, u32 ra, u32 *(sp+0x10), u32 s0, u32 the match frame *(gm+0x58)) per draw with a0 = the AI's
generator: from the percent roll 0x3640b0 (ra 0x3640cc) the word at sp+0x10 is its caller's ra and s0 the percent. Built like tools/record_ai_rally.py's counter hook (patched while paused, a few frames after the load)."""
import struct, sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine

VSYNC, GM_PTR, MT, DRAW = 0x1d5780, 0x422f80, 0x427130, 0x19f5c0
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


slot, want, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
p = Pine()
p.load_state(slot)
# the load lands late: a patch written before it is overwritten by the state's own RAM. Slot 5 loads at vsync
# ~7527 and its point starts at 7565: wait for the loaded vsync, then a few frames
while not 7500 <= p.read32(VSYNC) < 7550: time.sleep(0.01)
v0 = p.read32(VSYNC)
while p.read32(VSYNC) - v0 < 4: time.sleep(0.01)
gm0 = p.read32(GM_PTR)
if sys.argv[4:5] == ["shared"]: MT = p.read32(gm0 + 0x80)
a, b = p.read32(DRAW), p.read32(DRAW + 4)
# 0 lui t4; 1-2 li t5; 3 bne a0,t5 -> skip; 4 nop; 5 lw t6 ptr; 6-7 li t7 END; 8 sltu t7,t6,t7; 9 beq t7,0 -> skip;
# 10 nop; 11 lui t7 vsync; 12 lw t7; 13 sw t7; 14 sw ra; 15 lw t7 sp+0x10; 16 sw t7; 17 sw s0; 18-19 li t7 gm;
# 20 lw t7 (gm); 21 lw t7 +0x58; 22 sw t7; 23 addiu; 24 sw ptr; 25 skip: a; 26 b; 27 j; 28 nop
stub = [lui("t4", DATA >> 16), *li("t5", MT), i(5, "a0", "t5", 25 - 4), 0, lw("t6", DATA & 0xffff, "t4"),
        *li("t7", END), sltu("t7", "t6", "t7"), i(4, "t7", "zero", 25 - 10), 0,
        lui("t7", VSYNC >> 16), lw("t7", VSYNC & 0xffff, "t7"), sw("t7", 0, "t6"), sw("ra", 4, "t6"),
        lw("t7", int(sys.argv[5], 16) if sys.argv[5:] else 0x10, "sp"), sw("t7", 8, "t6"), sw("s0", 12, "t6"),
        *li("t7", GM_PTR), lw("t7", 0, "t7"), lw("t7", 0x58, "t7"), sw("t7", 16, "t6"), addiu("t6", "t6", 20), sw("t6", DATA & 0xffff, "t4"), a, b, j(DRAW + 8), 0]
p.pause()
p.write32(DATA, BUF)
for n, w in enumerate(stub): p.write32(CODE + 4 * n, w)
p.write32(DRAW, j(CODE))
p.write32(DRAW + 4, 0)
p.resume()
v0 = p.read32(VSYNC)
try:
    while p.read32(VSYNC) - v0 < want and p.read32(GM_PTR) == gm0 and p.read32(DATA) < END - 0x1000:
        time.sleep(0.1)
finally:
    p.pause()
    p.write32(DRAW + 4, b)
    p.write32(DRAW, a)
    p.resume()
end = p.read32(DATA)
data = b"".join(p.read_block(x, 0x10000) for x in range(BUF, end, 0x10000))[: end - BUF]
open(out, "wb").write(data)
print("done", len(data) // 20, "draws over", p.read32(VSYNC) - v0, "frames; full" if end >= END - 0x1000 else "")
