#!/usr/bin/env python3
"""Log the doubles AI's receive and rally routines call by call (P11k) from a save-state load.
Usage: record_ai_rally.py <slot> <frames> <out.bin> [keep 1 call in N] [tags, e.g. 1,2,3] [speed scale].

Hooks like record_ai_search.py (entry stub snapshots the world and swaps the return address for an exit stub that
appends the entry and exit records; a counter hook on the generator 0x19f5c0 with the AI's MT at 0x427130 counts
draws and copies the generator block at its first draw). Tags: 1 receive 0x3cf7b0, 2 NET 0x3d02c0, 3 BASE 0x3d1320;
each is called as (ai, *stick out, *button out). Every Nth call is kept (all calls draw). The buffer is drained
while the game is paused, so runs aren't limited by the free RAM.

File: b"AIRL", u32 0, the generator block at the first draw (0x9c8 bytes), then records. Record (REC bytes fixed,
then a variable tail; exit tag |0x100):
0x00 u32 tag, size, vsync, draws so far; 0x10 a0 a1 a2, v0 (exit); 0x20 *a1 (4 words), 0x30 *a2, the frame
(*0x422f80)+0x58, the phase +0x55, the player count 0x422fa4; 0x40 0x423040..0x423070 (12 words: server 0x42304c,
0x423050, receiver 0x423054, last hitter 0x423058, shot count 0x423060); 0x70 the path stamp 0x427108, its count
0x427110, the stamina floor 0x3fc8d8, 0; 0x80 the AI (0x280 bytes); 0x300 the character record (*(ai+8))
+0xd0..+0x110; the player (*(ai+4)): 0x340 +0x12b0 (0x20), 0x360 +0x1370 (0x20), 0x380 +0x13f0 (0x10), 0x390
+0x3050 (0x10), 0x3a0 +0x38f0 (0x10), 0x3b0 +0x3a90 (0xd0), 0x480 +0x3d60 (0x20), 0x4a0 +0x3df0 (0x20), 0x4c0
+0x3ec0 (0x10), 0x4d0 +0x3f90 (0x20); 0x4f0 partner (*(ai+0xec)) +0x3d70, 0x500 opponents (*(ai+0xe4), *(ai+0xe8))
+0x3d70, 0x520 partner +0x12b0 (0x10), 0x530 partner +0x13f0 (0x10); 0x540 the ball (*(gm+0x88)) +0xe0 (0x10),
0x550 +0x220 (0x10); 0x560 the path object (*(gm+0xa4)) +0x50..+0x130 (shot records at +0x70, 0x30 per player);
0x640 the seen flags 0x427050 (0xc0); 0x700 the character record +0..+0x10; 0x710 the ball +0x8c0..+0x8d0; 0x720
partner +0x3fa0 (0x10); 0x730 the caller's stack quad at sp-0x20 (the NET/BASE stand scratch, read uninitialised).
Tail: entry records the path object's entries from its start (+0x58) to its end (+0x54), at most 180, 0x30 each;
exit records the AI's path copy 0x424e90, ai+0x3c entries."""
import os, struct, sys, time
from pine import Pine

VSYNC, GM_PTR, MT = 0x1d5780, 0x422f80, 0x427130
CODE, DATA, SNAP, SCRATCH, BUF, END = 0x1e02000, 0x1e00000, 0x1e01000, 0x1e10000, 0x1e30000, 0x1f80000
PTR, CNT = DATA, DATA + 4
D = DATA & 0xffff  # t4 = DATA >> 16 << 16 in the stubs: DATA words are at t4 + D + off (D + off < 0x8000: the offset is signed)
R = dict(zero=0, at=1, v0=2, v1=3, a0=4, a1=5, a2=6, a3=7, t0=8, t1=9, t2=10, t3=11, t4=12, t5=13, t6=14, t7=15,
         t8=24, t9=25, sp=29, ra=31)
ARGS = ["a0", "a1", "a2", "a3", "t0", "t1", "t2", "t3"]
REC, CACHE, MAXPATH = 0x780, 0x424e90, 180
HOOKS = {1: 0x3cf7b0, 2: 0x3d02c0, 3: 0x3d1320}
# HST_SINGLES=1 (P11l): the singles routines (receive 0x3c9a40, NET 0x3ca4e0, BASE 0x3cb130). The opponent
# (*(ai+0xd4)) +0x3d70 goes to 0x500, the player's +0x3e90 (0x10) to 0x4f0, the opponent's hand byte
# (*(*(*(opp+0x54)))+0x135, the aim choosers') at 0x521; no partner (0x510, 0x530, 0x720 zero).
SINGLES = os.environ.get("HST_SINGLES") == "1"
if SINGLES: HOOKS = {1: 0x3c9a40, 2: 0x3ca4e0, 3: 0x3cb130}
# (at, source: list of (base, offsets to load through), first offset, bytes)
PLAYER = [(0x340, 0x12b0, 0x20), (0x360, 0x1370, 0x20), (0x380, 0x13f0, 0x10), (0x390, 0x3050, 0x10),
          (0x3a0, 0x38f0, 0x10), (0x3b0, 0x3a90, 0xd0), (0x480, 0x3d60, 0x20), (0x4a0, 0x3df0, 0x20),
          (0x4c0, 0x3ec0, 0x10), (0x4d0, 0x3f90, 0x20)]
REGIONS = ([(0x40, ("abs", 0x423040), 0, 0x30), (0x80, ("ai",), 0, 0x280), (0x300, ("ai", 8), 0xd0, 0x40)]
           + [(at, ("ai", 4), off, n) for at, off, n in PLAYER]
           + [(0x4f0, ("ai", 0xec), 0x3d70, 0x10), (0x500, ("ai", 0xe4), 0x3d70, 0x10), (0x510, ("ai", 0xe8), 0x3d70, 0x10),
              (0x520, ("ai", 0xec), 0x12b0, 0x10), (0x530, ("ai", 0xec), 0x13f0, 0x10),
              (0x540, ("abs", GM_PTR, 0, 0x88), 0xe0, 0x10), (0x550, ("abs", GM_PTR, 0, 0x88), 0x220, 0x10),
              (0x560, ("abs", GM_PTR, 0, 0xa4), 0x50, 0xe0), (0x640, ("abs", 0x427050), 0, 0xc0),
              (0x700, ("ai", 8), 0, 0x10), (0x710, ("abs", GM_PTR, 0, 0x88), 0x8c0, 0x10),
              (0x720, ("ai", 0xec), 0x3fa0, 0x10), (0x730, ("sp",), -0x20, 0x10)])
if SINGLES:
    REGIONS = [r for r in REGIONS if not (r[1][0] == "ai" and r[1][1:2] and r[1][1] in (0xe4, 0xe8, 0xec))] + [
        (0x4f0, ("ai", 4), 0x3e90, 0x10), (0x500, ("ai", 0xd4), 0x3d70, 0x10),
        (0x520, ("ai", 0xd4, 0x54, 0, 0), 0x134, 0x10)]


def i(op, rs, rt, imm): return op << 26 | R[rs] << 21 | R[rt] << 16 | imm & 0xffff
def lw(rt, off, rs): return i(0x23, rs, rt, off)
def lbu(rt, off, rs): return i(0x24, rs, rt, off)
def sw(rt, off, rs): return i(0x2b, rs, rt, off)
def lui(rt, imm): return i(0x0f, "zero", rt, imm)
def addiu(rt, rs, imm): return i(0x09, rs, rt, imm)
def ori(rt, rs, imm): return i(0x0d, rs, rt, imm)
def sll(rd, rt, sa): return R[rt] << 16 | R[rd] << 11 | sa << 6
def srl(rd, rt, sa): return R[rt] << 16 | R[rd] << 11 | sa << 6 | 0x02
def addu(rd, rs, rt): return R[rs] << 21 | R[rt] << 16 | R[rd] << 11 | 0x21
def subu(rd, rs, rt): return R[rs] << 21 | R[rt] << 16 | R[rd] << 11 | 0x23
def sltu(rd, rs, rt): return R[rs] << 21 | R[rt] << 16 | R[rd] << 11 | 0x2b
def slt(rd, rs, rt): return R[rs] << 21 | R[rt] << 16 | R[rd] << 11 | 0x2a
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


def save(tag): return DATA + 0x100 + 0x40 * tag  # ra, a0..a2, size, 2 temps


def ram_check(reg, bad):
    """Branch to `bad` unless reg is in 0x100000..0x2000000; uses t6."""
    return [*li("t6", 0x100000), sltu("t6", reg, "t6"), br(5, "t6", "zero", bad), 0,
            *li("t6", 0x2000000), sltu("t6", reg, "t6"), br(4, "t6", "zero", bad), 0]


def record(tag, size, exit):
    """Build a record into t5 from the saved arguments; t4 = DATA; uses t6..t9, v1, t2."""
    s = save(tag) & 0xffff
    p = [addiu("t6", "zero", tag | (0x100 if exit else 0)), sw("t6", 0, "t5"), sw(size, 4, "t5"),
         *li("t6", VSYNC), lw("t6", 0, "t6"), sw("t6", 8, "t5"), lw("t6", CNT & 0xffff, "t4"), sw("t6", 12, "t5")]
    for k in range(3):
        p += [lw("t6", s + 4 + 4 * k, "t4"), sw("t6", 0x10 + 4 * k, "t5")]
    p += [sw("v0" if exit else "zero", 0x1c, "t5")]
    p += [lw("t6", s + 8, "t4")] + copy("t6", 4, 0x20) + [lw("t6", s + 12, "t4"), lw("t6", 0, "t6"), sw("t6", 0x30, "t5")]
    p += [*li("t6", GM_PTR), lw("t6", 0, "t6"), lw("t7", 0x58, "t6"), sw("t7", 0x34, "t5"), lbu("t7", 0x55, "t6"),
          sw("t7", 0x38, "t5"), *li("t6", 0x422fa4), lw("t6", 0, "t6"), sw("t6", 0x3c, "t5")]
    for a, at in [(0x427108, 0x70), (0x427110, 0x74), (0x3fc8d8, 0x78)]:
        p += [*li("t6", a), lw("t6", 0, "t6"), sw("t6", at, "t5")]
    p += [sw("zero", 0x7c, "t5")]
    for at, src, off, n in REGIONS:
        if src[0] == "ai":
            p += [lw("t6", s + 4, "t4")] + [lw("t6", o, "t6") for o in src[1:]]
        elif src[0] == "sp":
            p += [addiu("t6", "sp", 0)]
        else:
            p += li("t6", src[1]) + [lw("t6", o, "t6") for o in src[2:]]
        p += [addiu("t6", "t6", off)] + copy("t6", n // 4, at)
    return p


def exit_at(tag): return CODE + 0x2000 * tag + 0x1000


def entry(tag, first, second):
    s = save(tag) & 0xffff
    busy = D + 0x34 + 4 * tag
    p = [lui("t4", DATA >> 16), lw("t6", busy, "t4"), br(5, "t6", "zero", "bad"), 0]
    # a nested call (busy) or one whose pointers aren't in RAM runs unhooked and is counted
    p += ram_check("a0", "bad")
    for o in (4, 8, 0xd4) if SINGLES else (4, 8, 0xe4, 0xe8, 0xec):
        p += [lw("t7", o, "a0")] + ram_check("t7", "bad")
    for o in (0x88, 0xa4):
        p += [*li("t7", GM_PTR), lw("t7", 0, "t7"), lw("t7", o, "t7")] + ram_check("t7", "bad")
    p += [addiu("t6", "zero", 1), sw("t6", busy, "t4"), sw("ra", s, "t4")] + [sw(r, s + 4 + 4 * k, "t4") for k, r in enumerate(ARGS[:3])]
    # the path object's entries: t2 = first, t3 = count (0..MAXPATH)
    p += [*li("t6", GM_PTR), lw("t6", 0, "t6"), lw("t6", 0xa4, "t6"), lw("t7", 0x58, "t6"), lw("t8", 0x54, "t6"),
          subu("t3", "t8", "t7"), slt("t9", "t3", "zero"), br(4, "t9", "zero", "pos"), 0, addiu("t3", "zero", 0), "pos",
          addiu("t9", "zero", MAXPATH + 1), slt("t9", "t3", "t9"), br(5, "t9", "zero", "fits"), 0, addiu("t3", "zero", MAXPATH), "fits",
          lw("t2", 0x50, "t6"), sll("t8", "t7", 5), sll("t7", "t7", 4), addu("t7", "t7", "t8"), addu("t2", "t2", "t7")]
    # size: REC + count · 0x30
    p += [sll("t7", "t3", 5), sll("t6", "t3", 4), addu("t6", "t6", "t7"), addiu("at", "t6", REC), sw("at", s + 0x10, "t4"),
          *li("t5", SCRATCH + 0x8000 * tag)]
    p += [sw("t2", s + 0x14, "t4"), sw("t3", s + 0x18, "t4")]  # kept over record(): it uses t2
    p += record(tag, "at", False)
    p += [lw("t2", s + 0x14, "t4"), lw("t3", s + 0x18, "t4"), sll("t6", "t3", 3), sll("t3", "t3", 2), addu("t6", "t6", "t3"),
          br(4, "t6", "zero", "nopath"), 0, *li("t5", SCRATCH + 0x8000 * tag + REC)]
    p += copyn("t2", "t6", 0) + ["nopath"]
    p += [lw(r, s + 4 + 4 * k, "t4") for k, r in enumerate(ARGS[:3])]
    p += [*li("ra", exit_at(tag)), first, second, j(HOOKS[tag] + 8), 0]
    p += ["bad", sw("a0", D + 0x20, "t4"), sw("ra", D + 0x24, "t4"), sw("a1", D + 0x28, "t4"), sw("sp", D + 0x2c, "t4"),
          lw("t6", D + 0x30, "t4"), addiu("t6", "t6", 1), sw("t6", D + 0x30, "t4"), first, second, j(HOOKS[tag] + 8), 0]
    return p


def leave(tag, every):
    scratch, s = SCRATCH + 0x8000 * tag, save(tag) & 0xffff
    p = [lui("t4", DATA >> 16), sw("zero", D + 0x34 + 4 * tag, "t4"), lw("t6", D + 8 + 4 * tag, "t4"), addiu("t6", "t6", 1),
         sw("t6", D + 8 + 4 * tag, "t4"), addiu("t7", "zero", every), br(5, "t6", "t7", "done"), 0, sw("zero", D + 8 + 4 * tag, "t4")]
    # room for the entry, the exit and its path copy? else count a drop
    p += [lw("t5", PTR & 0xffff, "t4"), *li("t6", END - 0x8000), sltu("t6", "t5", "t6"), br(5, "t6", "zero", "room"), 0,
          lw("t6", D + 0x18, "t4"), addiu("t6", "t6", 1), sw("t6", D + 0x18, "t4"), br(4, "zero", "zero", "done"), 0, "room"]
    p += [lw("at", s + 0x10, "t4"), *li("t2", scratch), srl("t3", "at", 2)]
    p += copyn("t2", "t3", 0) + [addu("t5", "t5", "at")]
    # exit size: REC + ai+0x3c · 0x30 (0..MAXPATH)
    p += [lw("t6", s + 4, "t4"), lw("t3", 0x3c, "t6"), slt("t9", "t3", "zero"), br(4, "t9", "zero", "pos"), 0, addiu("t3", "zero", 0), "pos",
          addiu("t9", "zero", MAXPATH + 1), slt("t9", "t3", "t9"), br(5, "t9", "zero", "fits"), 0, addiu("t3", "zero", MAXPATH), "fits",
          sw("t3", s + 0x18, "t4"), sll("t7", "t3", 5), sll("t6", "t3", 4), addu("t6", "t6", "t7"), addiu("at", "t6", REC)]
    p += record(tag, "at", True)
    p += [lw("t3", s + 0x18, "t4"), sll("t6", "t3", 3), sll("t3", "t3", 2), addu("t6", "t6", "t3"),
          br(4, "t6", "zero", "nopath"), 0, addiu("t5", "t5", REC), *li("t2", CACHE)] + copyn("t2", "t6", 0) + [addiu("t5", "t5", -REC), "nopath"]
    p += [addu("t5", "t5", "at"), sw("t5", PTR & 0xffff, "t4"), "done", lw("ra", s, "t4"), jr("ra"), 0]
    return p


def counter(first, second):
    return [lui("t4", DATA >> 16), *li("t5", MT), br(5, "a0", "t5", "skip"), 0,
            lw("t6", CNT & 0xffff, "t4"), br(5, "t6", "zero", "inc"), 0, *li("t5", SNAP)] + copy("a0", 0x9c8 // 4, 0) + [
            "inc", lw("t6", CNT & 0xffff, "t4"), addiu("t6", "t6", 1), sw("t6", CNT & 0xffff, "t4"),
            "skip", first, second, j(0x19f5c0 + 8), 0]


slot, want, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
every = int(sys.argv[4]) if len(sys.argv) > 4 else 1
tags = [int(t) for t in sys.argv[5].split(",")] if len(sys.argv) > 5 else list(HOOKS)
slow = float(sys.argv[6]) if len(sys.argv) > 6 else 1.0  # scales every player's speed stat (+0x1374): missed balls
p = Pine()
p.load_state(slot)
time.sleep(1)  # the load lands late: a patch written before it is overwritten by the state's own RAM
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


def drain():
    end = p.read32(PTR)
    data = b"".join(p.read_block(a, 0x10000) for a in range(BUF, end, 0x10000))[: end - BUF]  # one big batch times out
    p.write32(PTR, BUF)
    return data


p.pause()  # patch whole: a call between the jump and its delay slot would run half a hook
p.write32(PTR, BUF)
for k in range(1, 0x80): p.write32(DATA + 4 * k, 0)
for tag in tags:
    words = assemble(leave(tag, every))
    assert len(words) * 4 <= 0x1000
    for n, w in enumerate(words): p.write32(exit_at(tag) + 4 * n, w)
patch(0x19f5c0, CODE, counter)
for tag in tags:
    patch(HOOKS[tag], CODE + 0x2000 * tag, lambda a, b, tag=tag: entry(tag, a, b))
for k in range(p.read32(0x422fa4)):
    if slow != 1.0:
        pl = p.read32(gm0 + 0xa8 + 4 * k)
        p.write32(pl + 0x1374, struct.unpack("<I", struct.pack("<f", struct.unpack("<f", struct.pack("<I", p.read32(pl + 0x1374)))[0] * slow))[0])
p.resume()
v0 = p.read32(VSYNC)
chunks = []
try:
    while p.read32(VSYNC) - v0 < want and p.read32(GM_PTR) == gm0:
        time.sleep(0.1)
        if p.read32(PTR) > BUF + (END - BUF) // 3:
            p.pause()
            chunks.append(drain())
            p.resume()
finally:
    p.pause()
    for at, a, b in reversed(installed):
        p.write32(at + 4, b)
        p.write32(at, a)
    p.resume()
time.sleep(0.2)
chunks.append(drain())
data = b"".join(chunks)
open(out, "wb").write(b"AIRL" + bytes(4) + p.read_block(SNAP, 0x9c8) + data)
print("unhooked calls (last a0 ra a1 sp, count, busy):", [hex(p.read32(DATA + 0x20 + 4 * k)) for k in range(8)])
print("dropped", p.read32(DATA + 0x18), "done", len(data), "bytes over", p.read32(VSYNC) - v0, "frames, draws", p.read32(CNT))
