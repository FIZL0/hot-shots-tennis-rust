#!/usr/bin/env python3
"""Log the computer players' contact searches call by call (P11j) from a save-state load.
Usage: record_ai_search.py <slot> <frames> <out.bin> [keep 1 call in N].

Hooks like record_ai_serve.py (entry stub snapshots the call, swaps the return address for an exit stub that
appends entry and exit records; a counter hook on the generator 0x19f5c0 with the AI's MT at 0x427130 counts draws
and copies the generator block at its first draw). The searches take eight register arguments (a0..a3, t0..t3) and
more on the stack, so the stubs here keep off t0..t3 and restore them before entering the function.

Hooks (tag): 1 the reach search 0x360150 (ai, start, min bounces, *stand, *ball, *frames, *index, all; stack: body),
2 the tiered search 0x360910 (ai, start, tier, min bounces, *stand, *ball, *frames, *index; stack: all, no reach,
no height). Every Nth call is kept (all calls draw).

File: b"AISR", u32 0, the generator block at the first draw (0x9c8 bytes), then records. Record: u32 tag (exit
|0x100), u32 size, u32 vsync, u32 draws so far; 0x10 a0..a3 t0..t3; 0x30 the caller's stack +0, +8, +0x10; 0x3c v0
(exit); 0x40 the AI +0..+0x40; 0x80 the character record (*(ai+8)) +0xd0..+0x110; the player (*(ai+4)) 0xc0
+0x12b0..+0x12d0, 0xe0 +0x1370..+0x1390, 0x100 +0x3050..+0x3060, 0x110 +0x3d70..+0x3d80, 0x120 +0x3df0..+0x3e00,
0x130 +0x3fa0..+0x3fb0; 0x140 the player count 0x422fa4, the match phase (*0x422f80)+0x55, the stamina floor
0x3fc8d8, the strong side (*(ai+8)+4); 0x150 *stand (4 words), 0x160 *ball (4), 0x170 *frames, 0x174 *index;
0x180 the seen flags 0x427050 (0xe0 bytes). Entry records then carry the path 0x424e90 entries 0..ai+0x3c (0x30
each) at 0x260."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, MT = 0x1d5780, 0x422f80, 0x427130
CODE, DATA, SNAP, SCRATCH, BUF, END = 0x1e00000, 0x1e07000, 0x1e08000, 0x1e10000, 0x1e30000, 0x1f80000
PTR, CNT = DATA, DATA + 4
D = DATA & 0xffff  # t4 = DATA >> 16 << 16 in the stubs: DATA words are at t4 + D + off
R = dict(zero=0, at=1, v0=2, v1=3, a0=4, a1=5, a2=6, a3=7, t0=8, t1=9, t2=10, t3=11, t4=12, t5=13, t6=14, t7=15,
         t8=24, t9=25, sp=29, ra=31)
ARGS = ["a0", "a1", "a2", "a3", "t0", "t1", "t2", "t3"]
REC, FLAGS = 0x260, 0x427050
# tag: (address, the out pointers' argument numbers: stand, ball, frames, index)
HOOKS = {1: (0x360150, (3, 4, 5, 6)), 2: (0x360910, (4, 5, 6, 7))}


def i(op, rs, rt, imm): return op << 26 | R[rs] << 21 | R[rt] << 16 | imm & 0xffff
def lw(rt, off, rs): return i(0x23, rs, rt, off)
def lbu(rt, off, rs): return i(0x24, rs, rt, off)
def sw(rt, off, rs): return i(0x2b, rs, rt, off)
def lui(rt, imm): return i(0x0f, "zero", rt, imm)
def addiu(rt, rs, imm): return i(0x09, rs, rt, imm)
def ori(rt, rs, imm): return i(0x0d, rs, rt, imm)
def sll(rd, rt, sa): return R[rt] << 16 | R[rd] << 11 | sa << 6
def addu(rd, rs, rt): return R[rs] << 21 | R[rt] << 16 | R[rd] << 11 | 0x21
def sltu(rd, rs, rt): return R[rs] << 21 | R[rt] << 16 | R[rd] << 11 | 0x2b
def j(a): return 0x02 << 26 | (a >> 2) & 0x3ffffff
def jr(rs): return R[rs] << 21 | 0x08
def li(rt, v): return [lui(rt, v >> 16), ori(rt, rt, v & 0xffff)]
def br(op, rs, rt, label): return ("br", op, rs, rt, label)  # beq 4 / bne 5; add the delay slot yourself


def assemble(prog):
    at, labels = 0, {}
    for x in prog:
        if isinstance(x, str): labels[x] = at
        else: at += 1
    out = []
    for x in prog:
        if isinstance(x, str): continue
        if isinstance(x, tuple):
            _, op, rs, rt, label = x
            x = i(op, rs, rt, labels[label] - len(out) - 1)
        out.append(x)
    return out


def copyn(src, nreg, at):
    """nreg (a register, > 0) words from register src to the record's +at (t5 the record); uses t7..t9, v1."""
    lbl = f"c{copyn.n}"
    copyn.n += 1
    return [addiu("t7", src, 0), addiu("t8", "t5", at), addiu("t9", nreg, 0), lbl,
            lw("v1", 0, "t7"), sw("v1", 0, "t8"), addiu("t7", "t7", 4), addiu("t8", "t8", 4), addiu("t9", "t9", -1),
            br(5, "t9", "zero", lbl), 0]
copyn.n = 0


def copy(src, n, at): return [addiu("t9", "zero", n)] + copyn(src, "t9", at)


def save(tag): return DATA + 0x40 * tag  # ra, a0..a3, t0..t3, stack +0, +8, +0x10


def record(tag, size, exit):
    """Build a record into t5 from the saved arguments; t4 = DATA; uses t6..t9, v1."""
    s = save(tag) & 0xffff
    p = [addiu("t6", "zero", tag | (0x100 if exit else 0)), sw("t6", 0, "t5"), sw(size, 4, "t5"),
         *li("t6", VSYNC), lw("t6", 0, "t6"), sw("t6", 8, "t5"), lw("t6", CNT & 0xffff, "t4"), sw("t6", 12, "t5")]
    for k in range(11):
        p += [lw("t6", s + 4 + 4 * k, "t4"), sw("t6", 0x10 + 4 * k, "t5")]
    p += [sw("v0" if exit else "zero", 0x3c, "t5"), lw("t6", s + 4, "t4")] + copy("t6", 0x10, 0x40)
    p += [lw("t6", s + 4, "t4"), lw("t6", 8, "t6"), addiu("t6", "t6", 0xd0)] + copy("t6", 0x10, 0x80)
    for off, at in [(0x12b0, 0xc0), (0x12c0, 0xd0), (0x1370, 0xe0), (0x1380, 0xf0), (0x3050, 0x100), (0x3d70, 0x110),
                    (0x3df0, 0x120), (0x3fa0, 0x130)]:
        p += [lw("t6", s + 4, "t4"), lw("t6", 4, "t6"), addiu("t6", "t6", off)] + copy("t6", 4, at)
    p += [*li("t6", 0x422fa4), lw("t6", 0, "t6"), sw("t6", 0x140, "t5"),
          *li("t6", GM_PTR), lw("t6", 0, "t6"), lbu("t6", 0x55, "t6"), sw("t6", 0x144, "t5"),
          *li("t6", 0x3fc8d8), lw("t6", 0, "t6"), sw("t6", 0x148, "t5"),
          lw("t6", s + 4, "t4"), lw("t6", 8, "t6"), lw("t6", 4, "t6"), sw("t6", 0x14c, "t5")]
    stand, ball, frames, index = HOOKS[tag][1]
    for arg, n, at in [(stand, 4, 0x150), (ball, 4, 0x160), (frames, 1, 0x170), (index, 1, 0x174)]:
        p += [lw("t6", s + 4 + 4 * arg, "t4")] + copy("t6", n, at)
    p += li("t6", FLAGS) + copy("t6", 0xe0 // 4, 0x180)
    return p


def exit_at(tag): return CODE + 0x2000 * tag + 0x1000


def entry(tag, first, second):
    s = save(tag) & 0xffff
    busy = D + 0x34 + 4 * tag
    p = [lui("t4", DATA >> 16), lw("t6", busy, "t4"), br(5, "t6", "zero", "bad"), 0]
    # a nested call (busy) or one whose AI or player pointer isn't in RAM runs unhooked and is counted
    p += [*li("t6", 0x100000), sltu("t6", "a0", "t6"), br(5, "t6", "zero", "bad"), 0,
          *li("t6", 0x2000000), sltu("t6", "a0", "t6"), br(4, "t6", "zero", "bad"), 0,
          lw("t7", 4, "a0"), *li("t6", 0x100000), sltu("t6", "t7", "t6"), br(5, "t6", "zero", "bad"), 0,
          *li("t6", 0x2000000), sltu("t6", "t7", "t6"), br(4, "t6", "zero", "bad"), 0,
          addiu("t6", "zero", 1), sw("t6", busy, "t4"), sw("ra", s, "t4")] + [sw(r, s + 4 + 4 * k, "t4") for k, r in enumerate(ARGS)]
    p += [lw("t6", 0, "sp"), sw("t6", s + 0x24, "t4"), lw("t6", 8, "sp"), sw("t6", s + 0x28, "t4"),
          lw("t6", 0x10, "sp"), sw("t6", s + 0x2c, "t4")]
    # size: REC + end · 0x30
    p += [lw("t6", 0x3c, "a0"), sll("t7", "t6", 5), sll("t6", "t6", 4), addu("t6", "t6", "t7"),
          addiu("t6", "t6", REC), sw("t6", s + 0x30, "t4"), *li("t5", SCRATCH + 0x8000 * tag), addiu("at", "t6", 0)]
    p += record(tag, "at", False)
    p += [lw("t6", s + 4, "t4"), lw("t6", 0x3c, "t6"), sll("t7", "t6", 3), sll("t6", "t6", 2), addu("t6", "t6", "t7"),
          br(4, "t6", "zero", "nopath"), 0, *li("t5", SCRATCH + 0x8000 * tag + REC), *li("t2", 0x424e90)]
    p += copyn("t2", "t6", 0) + ["nopath"]
    p += [lw(r, s + 4 + 4 * k, "t4") for k, r in enumerate(ARGS)]
    p += [*li("ra", exit_at(tag)), first, second, j(HOOKS[tag][0] + 8), 0]
    p += ["bad", sw("a0", D + 0x20, "t4"), sw("ra", D + 0x24, "t4"), sw("a1", D + 0x28, "t4"), sw("sp", D + 0x2c, "t4"),
          lw("t6", D + 0x30, "t4"), addiu("t6", "t6", 1), sw("t6", D + 0x30, "t4"), first, second, j(HOOKS[tag][0] + 8), 0]
    return p


def leave(tag, every):
    scratch, s = SCRATCH + 0x8000 * tag, save(tag) & 0xffff
    p = [lui("t4", DATA >> 16), sw("zero", D + 0x34 + 4 * tag, "t4"), lw("t6", D + 8 + 4 * tag, "t4"), addiu("t6", "t6", 1), sw("t6", D + 8 + 4 * tag, "t4"),
         addiu("t7", "zero", every), br(5, "t6", "t7", "done"), 0, sw("zero", D + 8 + 4 * tag, "t4")]
    # room left? (the Python side stops at END - 0x40000)
    p += [lw("t5", PTR & 0xffff, "t4"), *li("t6", END - 0x10000), sltu("t6", "t5", "t6"), br(4, "t6", "zero", "done"), 0]
    p += [lw("at", s + 0x30, "t4"), *li("t2", scratch)]
    # words = size / 4
    p += [R["at"] << 16 | R["t3"] << 11 | 2 << 6 | 0x02]  # srl t3, at, 2
    p += copyn("t2", "t3", 0) + [addu("t5", "t5", "at"), addiu("at", "zero", REC)] + record(tag, "at", True)
    p += [addiu("t5", "t5", REC), sw("t5", PTR & 0xffff, "t4"), "done", lw("ra", s, "t4"), jr("ra"), 0]
    return p


def counter(first, second):
    return [lui("t4", DATA >> 16), *li("t5", MT), br(5, "a0", "t5", "skip"), 0,
            lw("t6", CNT & 0xffff, "t4"), br(5, "t6", "zero", "inc"), 0, *li("t5", SNAP)] + copy("a0", 0x9c8 // 4, 0) + [
            "inc", lw("t6", CNT & 0xffff, "t4"), addiu("t6", "t6", 1), sw("t6", CNT & 0xffff, "t4"),
            "skip", first, second, j(0x19f5c0 + 8), 0]


slot, want, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
every = int(sys.argv[4]) if len(sys.argv) > 4 else 1
p = Pine()
p.load_state(slot)
time.sleep(1)  # the load lands late: a patch written before it is overwritten by the state's own RAM, half or whole
v0 = p.read32(VSYNC)
while p.read32(VSYNC) - v0 < 5: time.sleep(0.05)
gm0 = p.read32(GM_PTR)
installed = []


def patch(at, stub, build):
    a, b = p.read32(at), p.read32(at + 4)
    words = assemble(build(a, b))
    assert len(words) * 4 <= 0x1000, hex(at)
    for n, w in enumerate(words): p.write32(stub + 4 * n, w)
    p.write32(at, j(stub))
    p.write32(at + 4, 0)
    installed.append((at, a, b))


p.pause()  # patch whole: a call between the jump and its delay slot would run half a hook
p.write32(PTR, BUF)
for k in range(1, 16): p.write32(DATA + 4 * k, 0)
for tag in HOOKS:
    words = assemble(leave(tag, every))
    assert len(words) * 4 <= 0x1000
    for n, w in enumerate(words): p.write32(exit_at(tag) + 4 * n, w)
patch(0x19f5c0, CODE, counter)
for tag, (at, _) in HOOKS.items():
    patch(at, CODE + 0x2000 * tag, lambda a, b, tag=tag: entry(tag, a, b))
p.resume()
v0 = p.read32(VSYNC)
try:
    while p.read32(VSYNC) - v0 < want and p.read32(PTR) < END - 0x40000 and p.read32(GM_PTR) == gm0:
        time.sleep(0.5)
finally:
    p.pause()
    for at, a, b in reversed(installed):
        p.write32(at + 4, b)
        p.write32(at, a)
    p.resume()
time.sleep(0.2)
end = p.read32(PTR)
data = b"".join(p.read_block(a, 0x10000) for a in range(BUF, end, 0x10000))[: end - BUF]  # one big batch times out
open(out, "wb").write(b"AISR" + bytes(4) + p.read_block(SNAP, 0x9c8) + data)
print("unhooked calls (last a0 ra a1 sp, count, busy):", [hex(p.read32(DATA + 0x20 + 4 * k)) for k in range(8)])
print("done", end - BUF, "bytes over", p.read32(VSYNC) - v0, "frames, draws", p.read32(CNT))
