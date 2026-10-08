#!/usr/bin/env python3
"""Log every strong-side decision of the AI's two contact searches (P11e4) from a save-state load.
Usage: record_ai_side.py <slot> <frames> <out.bin>.

The decision (both sides of the ball found: keep the nearer stand spot, or run round to the strong side) leaves no
trace in RAM, so this patches the running game: a jump at the decision's first instruction and at the searches'
common return goes to a stub in free RAM (0x1e00000) that appends a 0x50-byte record to a buffer and runs the
replaced instruction. The patches are removed at the end (the save state is never written).

Record: u32 tag (0x10910/0x10150 before the decision in the 360910/360150 search, 0x20910/0x20150 at its return),
u32 vsync, u32 AI, i32 minus index (stand at ball x − reach·side), i32 plus index; before: f32 reach, f32 width
(court half width + row extend), f32 side, f32 player x, f32 player z, u32 roll (0 or 1, after the beside-human
clear), u32 strong (player +0x12dc), f32 x/z of the minus and plus path entries, f32 player +0x3050, u32 row
pointer, i32 AI +0x28."""
import struct, sys, time
from pine import Pine

VSYNC, STUB, PTR, BUF, END = 0x1d5780, 0x1e00000, 0x1e00f00, 0x1e01000, 0x1f80000
REC = 0x50
R = dict(zero=0, at=1, v0=2, v1=3, a0=4, a1=5, a2=6, a3=7, t0=8, s4=20, sp=29)


def i(op, rs, rt, imm): return op << 26 | R[rs] << 21 | R[rt] << 16 | imm & 0xffff
def lw(rt, off, rs): return i(0x23, rs, rt, off)
def lbu(rt, off, rs): return i(0x24, rs, rt, off)
def sw(rt, off, rs): return i(0x2b, rs, rt, off)
def swc1(ft, off, rs): return 0x39 << 26 | R[rs] << 21 | ft << 16 | off & 0xffff
def lui(rt, imm): return i(0x0f, "zero", rt, imm)
def addiu(rt, rs, imm): return i(0x09, rs, rt, imm)
def sll(rd, rt, sa): return R[rt] << 16 | R[rd] << 11 | sa << 6
def addu(rd, rs, rt): return R[rs] << 21 | R[rt] << 16 | R[rd] << 11 | 0x21
def j(a): return 0x02 << 26 | (a >> 2) & 0x3ffffff


def head(tag):
    """a1 = the buffer's write pointer; tag, vsync and AI stored (uses a1, a3, t0)."""
    return [lui("t0", PTR >> 16), lw("a1", PTR & 0xffff, "t0"), addiu("a3", "zero", tag & 0xffff),
            lui("t0", tag >> 16), addu("a3", "a3", "t0"), sw("a3", 0, "a1"),
            lui("a3", VSYNC >> 16), lw("a3", VSYNC & 0xffff, "a3"), sw("a3", 4, "a1"), sw("s4", 8, "a1")]


def tail():
    return [addiu("a1", "a1", REC), lui("t0", PTR >> 16), sw("a1", PTR & 0xffff, "t0")]


def entry(idx, at):
    """x and z of path entry `idx` (a register) at +at, +at+4."""
    return [sll("a3", idx, 1), addu("a3", "a3", idx), sll("a3", "a3", 4), lui("t0", 0x42), addu("a3", "a3", "t0"),
            lw("t0", 0x4e90, "a3"), sw("t0", at, "a1"), lw("t0", 0x4e98, "a3"), sw("t0", at + 4, "a1")]


def pre(tag, reach, width, side, pos, roll, strong, back):
    side_st = [lw("t0", side, "sp"), sw("t0", 28, "a1")] if isinstance(side, int) else [swc1(side[0], 28, "a1")]
    return (head(tag) + [sw("v1", 12, "a1"), sw("v0", 16, "a1"), swc1(reach, 20, "a1"), swc1(width, 24, "a1")]
            + side_st + [lw("t0", pos, "sp"), sw("t0", 32, "a1"), lw("t0", pos + 8, "sp"), sw("t0", 36, "a1"),
                         lbu("t0", roll, "sp"), sw("t0", 40, "a1"), lw("t0", strong, "sp"), sw("t0", 44, "a1")]
            + entry("v1", 48) + entry("v0", 56)
            + [lw("a3", 4, "s4"), lw("t0", 0x3050, "a3"), sw("t0", 64, "a1"), lw("t0", 0xc, "s4"), sw("t0", 68, "a1"),
               lw("t0", 0x28, "s4"), sw("t0", 72, "a1")]
            + tail() + [sll("a0", "v1", 1), j(back), 0])


def post(tag, minus, plus, back):
    return (head(tag) + [lw("t0", minus, "sp"), sw("t0", 12, "a1"), lw("t0", plus, "sp"), sw("t0", 16, "a1")]
            + tail() + [addiu("v0", "zero", 1), j(back), 0])


# (hook address, its instruction, stub): the hooked instruction's delay-slot neighbour runs first, then the stub
# runs the hooked one and jumps past both.
HOOKS = [
    (0x3610b4, sll("a0", "v1", 1), pre(0x10910, 30, 31, 0xdc, 0x120, 0xf0, 0xe0, 0x3610bc)),
    (0x361378, addiu("v0", "zero", 1), post(0x20910, 0x15c, 0x158, 0x361380)),
    (0x3605f4, sll("a0", "v1", 1), pre(0x10150, 21, 22, (23,), 0x110, 0xe0, 0xd0, 0x3605fc)),
    (0x3608a8, addiu("v0", "zero", 1), post(0x20150, 0x14c, 0x148, 0x3608b0)),
]

slot, want, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
p = Pine()
p.load_state(slot)
time.sleep(0.3)
for k, (at, word, stub) in enumerate(HOOKS):
    if p.read32(at) != word:
        sys.exit(f"hook {at:#x}: found {p.read32(at):#010x}, expected {word:#010x}")
    for n, w in enumerate(stub):
        p.write32(STUB + 0x200 * k + 4 * n, w)
p.write32(PTR, BUF)
for k, (at, _, _) in enumerate(HOOKS):
    p.write32(at, j(STUB + 0x200 * k))
v0 = p.read32(VSYNC)
try:
    while p.read32(VSYNC) - v0 < want and p.read32(PTR) < END - REC:
        time.sleep(0.5)
finally:
    for at, word, _ in HOOKS:
        p.write32(at, word)
end = p.read32(PTR)
data = p.read_block(BUF, (end - BUF + 7) & ~7)[: end - BUF]
open(out, "wb").write(data)
print("done", (end - BUF) // REC, "records over", p.read32(VSYNC) - v0, "frames")
