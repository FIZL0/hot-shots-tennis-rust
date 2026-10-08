#!/usr/bin/env python3
"""Log every aim choice of the AI (P11f) from a save-state load: the doubles chooser, or with `singles` the singles
return-of-serve and rally choosers. Usage: record_ai_aim.py <slot> <frames> <out.bin> [singles].

The choice leaves only its result in RAM (and draws the generator several times), so this patches the running game
as record_ai_side.py does: the choosers' first two instructions become a jump to a stub in free RAM (0x1e00000)
that appends a record, swaps the return address for an exit stub, runs the two instructions and jumps back; the
exit stub appends a second record and returns to the caller. The patches are removed at the end.

Record (0xc90 bytes): u32 tag (3 doubles, 1/2 singles receive/rally at the entry; 0x10 at the exit), u32 vsync,
u32 AI, u32 0; the AI object (0x280 bytes); at 0x290 f32 x, z of the player, its partner, the two opponents (+0xe4,
+0xe8); f32 its side (+0x12b0), u32 its formation byte (+0x13f4). Singles: the opponent (+0xd4) at 0x2a0; the three
reach heights the AI holds its contact against (+8 -> +0xdc/+0xe0/+0xe4) at 0x298, 0x29c, 0x2a8; u32 the
opponent's hand byte at 0x2ac. At 0x2c0 the generator's block (0x9c8 bytes: state, index at +0x9c4)."""
import struct, sys, time
from pine import Pine

VSYNC, STUB, PTR, SAVE, BUF, END = 0x1d5780, 0x1e00000, 0x1e00f00, 0x1e00f10, 0x1e01000, 0x1f80000
MT, REC = 0x427130, 0xc90
R = dict(zero=0, v0=2, a0=4, sp=29, ra=31, t0=8, t1=9, t2=10, t3=11, t4=12, t5=13, t6=14)


def i(op, rs, rt, imm): return op << 26 | R[rs] << 21 | R[rt] << 16 | imm & 0xffff
def lw(rt, off, rs): return i(0x23, rs, rt, off)
def lbu(rt, off, rs): return i(0x24, rs, rt, off)
def sw(rt, off, rs): return i(0x2b, rs, rt, off)
def lui(rt, imm): return i(0x0f, "zero", rt, imm)
def addiu(rt, rs, imm): return i(0x09, rs, rt, imm)
def ori(rt, rs, imm): return i(0x0d, rs, rt, imm)
def bne(rs, rt, words): return i(0x05, rs, rt, words)
def j(a): return 0x02 << 26 | (a >> 2) & 0x3ffffff
def jr(rs): return R[rs] << 21 | 0x08


def li(rt, v): return [lui(rt, v >> 16), ori(rt, rt, v & 0xffff)]


def copy(src, n, at):
    """n words from src (a register) to the record's +at (t1 the record); uses t2..t5."""
    return [addiu("t2", src, 0), addiu("t3", "t1", at), addiu("t4", "zero", n),
            lw("t5", 0, "t2"), sw("t5", 0, "t3"), addiu("t2", "t2", 4), addiu("t3", "t3", 4), addiu("t4", "t4", -1),
            bne("t4", "zero", -6), 0]


def record(tag, ai):
    """Append a record for the AI in register `ai` (not t1..t6)."""
    return ([lui("t0", PTR >> 16), lw("t1", PTR & 0xffff, "t0"), addiu("t2", "zero", tag), sw("t2", 0, "t1"),
             lui("t2", VSYNC >> 16), lw("t2", VSYNC & 0xffff, "t2"), sw("t2", 4, "t1"), sw(ai, 8, "t1"),
             sw("zero", 12, "t1")]
            + copy(ai, 0xa0, 0x10)
            + sum(([lw("t6", obj, ai), lw("t2", 0x3d70, "t6"), sw("t2", at, "t1"), lw("t2", 0x3d78, "t6"),
                    sw("t2", at + 4, "t1")] for obj, at in PLAYERS), [])
            + [lw("t6", 4, ai), lw("t2", 0x12b0, "t6"), sw("t2", 0x2b0, "t1"), lbu("t2", 0x13f4, "t6"),
               sw("t2", 0x2b4, "t1")]
            + ([] if not SINGLES else
               [lw("t6", 0xd4, ai), lw("t6", 0x54, "t6"), lw("t6", 0, "t6"), lw("t6", 0, "t6"), lbu("t2", 0x135, "t6"),
                sw("t2", 0x2ac, "t1"), lw("t6", 8, ai), lw("t2", 0xdc, "t6"), sw("t2", 0x298, "t1"),
                lw("t2", 0xe0, "t6"), sw("t2", 0x29c, "t1"), lw("t2", 0xe4, "t6"), sw("t2", 0x2a8, "t1")])
            + li("t6", MT) + copy("t6", 0x9c8 // 4, 0x2c0)
            + [lui("t0", PTR >> 16), lw("t1", PTR & 0xffff, "t0"), addiu("t1", "t1", REC), sw("t1", PTR & 0xffff, "t0")])


SINGLES = len(sys.argv) > 4 and sys.argv[4] == "singles"
# own, partner, the two opponents (singles: own, opponent): (pointer in the AI, record offset of x, z)
PLAYERS = [(4, 0x290), (0xd4, 0x2a0)] if SINGLES else [(4, 0x290), (0xec, 0x298), (0xe4, 0x2a0), (0xe8, 0x2a8)]
EXIT = STUB + 0x800


def entry(tag, at, first, second):
    return (record(tag, "a0") + [lui("t0", SAVE >> 16), sw("ra", SAVE & 0xffff, "t0"), sw("a0", (SAVE & 0xffff) + 4, "t0")]
            + li("ra", EXIT) + [first, second, j(at + 8), 0])


def leave():
    return ([lui("t0", SAVE >> 16), lw("t6", (SAVE & 0xffff) + 4, "t0")]
            # t6 is overwritten inside record, so the AI goes through a0's slot: a0 is free after the return
            + [addiu("a0", "t6", 0)] + record(0x10, "a0")
            + [lui("t0", SAVE >> 16), lw("ra", SAVE & 0xffff, "t0"), jr("ra"), 0])


HOOKS = [(0x3cc700, 1), (0x3cce80, 2)] if SINGLES else [(0x3d3080, 3)]

slot, want, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
p = Pine()
p.load_state(slot)
time.sleep(0.3)
orig = []
for k, (at, tag) in enumerate(HOOKS):
    a, b = p.read32(at), p.read32(at + 4)
    if a >> 16 != 0x27bd:
        sys.exit(f"hook {at:#x}: found {a:#010x}, expected addiu sp")
    orig.append((at, a, b))
    for n, w in enumerate(entry(tag, at, a, b)):
        p.write32(STUB + 0x400 * k + 4 * n, w)
for n, w in enumerate(leave()):
    p.write32(EXIT + 4 * n, w)
p.write32(PTR, BUF)
for k, (at, a, b) in enumerate(orig):
    # the jump first: until the nop lands its delay slot runs the second instruction twice, which is harmless
    p.write32(at, j(STUB + 0x400 * k))
    p.write32(at + 4, 0)
v0 = p.read32(VSYNC)
try:
    while p.read32(VSYNC) - v0 < want and p.read32(PTR) < END - 2 * REC:
        time.sleep(0.5)
finally:
    for at, a, b in orig:
        p.write32(at + 4, b)
        p.write32(at, a)
time.sleep(0.2)
end = p.read32(PTR)
data = p.read_block(BUF, (end - BUF + 7) & ~7)[: end - BUF]
open(out, "wb").write(data)
print("done", (end - BUF) // REC, "records over", p.read32(VSYNC) - v0, "frames")
