#!/usr/bin/env python3
"""Log the computer players' kind lock, serve choices and serve contact pick (P11i) from a save-state load.
Usage: record_ai_serve.py <slot> <frames> <out.bin>.

Patches the running game like record_ai_aim.py: the first two instructions of each hooked function jump to a stub
in free RAM that snapshots its arguments and the AI at entry, swaps the return address for an exit stub, and on
return appends (entry snapshot, exit record) to a buffer when the call is worth keeping. A counter hook on the
game's generator (0x19f5c0 with the AI's MT at 0x427130) counts draws and copies the generator's block (state,
index at +0x9c4) at its first draw, so a record carries only the draw count: the generator before the call is that
block advanced by the count. The patches are removed at the end.

Hooks (tag): 1 kind lock 0x364100 (singles; ai, stick*, button*) — kept when it draws or the player's +0x3ec4 is 1;
2 singles serve state 0x3c8f10 and 3 doubles 0x3cee70 (ai, stick*, button*) — kept when it draws, the AI's state
byte +0x55 changes or +0x3ec4 is 1; 4 serve contact pick 0x360010 (ai, toss, quick, *frame) — kept when it returns
0 (the press), with the ball path.

File: b"AISV", u32 0, the generator block at the first draw (0x9c8 bytes), then records. Record: u32 tag (exit
|0x100), u32 size, u32 vsync, u32 draws so far; at 0x10 a0..a3; 0x20 v0 (exit); 0x24 *a1 (4 words; hooks 1-3);
0x34 *a2 (hooks 1-3); 0x38 *a3 (hook 4); 0x40 the AI (0x280); player (*(ai+4)) 0x2c0 +0x12b0..+0x12c0, 0x2d0
+0x3d60..+0x3d80, 0x2f0 +0x3ec0..+0x3ed0, 0x300 +0x3f90..+0x3fa0; 0x310 globals 0x423048..0x423068; 0x330
0x316600..0x316610; 0x340 reach block (*(ai+8)) +0xf0..+0x110; 0x360 the player's hand byte. Hook 4's entry: 0x380
the path at 0x424e90 (0x60 steps of 0x30)."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, MT = 0x1d5780, 0x422f80, 0x427130
CODE, DATA, SNAP, SCRATCH, BUF, END = 0x1e00000, 0x1e07000, 0x1e08000, 0x1e10000, 0x1e20000, 0x1f80000
PTR, CNT = DATA, DATA + 4
R = dict(zero=0, v0=2, a0=4, a1=5, a2=6, a3=7, t0=8, t1=9, t2=10, t3=11, t4=12, t5=13, t6=14, t7=15, ra=31)
REC, PATH = 0x380, 0x1200
# tag: (address, deref a1/a2/a3, keep mask: 1 draws, 2 returns 0, 4 player +0x3ec4 == 1, 8 state byte changed, path)
HOOKS = {1: (0x364100, "12", 1 | 4, False), 2: (0x3c8f10, "12", 1 | 4 | 8, False),
         3: (0x3cee70, "12", 1 | 4 | 8, False), 4: (0x360010, "3", 2, True)}


def i(op, rs, rt, imm): return op << 26 | R[rs] << 21 | R[rt] << 16 | imm & 0xffff
def lw(rt, off, rs): return i(0x23, rs, rt, off)
def lbu(rt, off, rs): return i(0x24, rs, rt, off)
def sw(rt, off, rs): return i(0x2b, rs, rt, off)
def lui(rt, imm): return i(0x0f, "zero", rt, imm)
def addiu(rt, rs, imm): return i(0x09, rs, rt, imm)
def ori(rt, rs, imm): return i(0x0d, rs, rt, imm)
def j(a): return 0x02 << 26 | (a >> 2) & 0x3ffffff
def jr(rs): return R[rs] << 21 | 0x08
def li(rt, v): return [lui(rt, v >> 16), ori(rt, rt, v & 0xffff)]
def br(op, rs, rt, label): return ("br", op, rs, rt, label)  # beq 4 / bne 5 to a label; add the delay slot yourself


def assemble(base, prog):
    """Words from a list of ints, branch tuples and label strings."""
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


def copy(src, n, at):
    """n words from register src to the record's +at (t1 the record); uses t2..t5."""
    lbl = f"c{copy.n}"
    copy.n += 1
    return [addiu("t2", src, 0), addiu("t3", "t1", at), addiu("t4", "zero", n), lbl,
            lw("t5", 0, "t2"), sw("t5", 0, "t3"), addiu("t2", "t2", 4), addiu("t3", "t3", 4), addiu("t4", "t4", -1),
            br(5, "t4", "zero", lbl), 0]
copy.n = 0


def save(tag): return DATA + 0x10 + 0x20 * tag  # ra, a0, a1, a2, a3


def record(tag, size, exit):
    """Build a record into t1 from the saved arguments; uses t0, t2..t7."""
    s = save(tag) & 0xffff
    addr, deref, _, _ = HOOKS[tag]
    p = [addiu("t2", "zero", tag | (0x100 if exit else 0)), sw("t2", 0, "t1"), addiu("t2", "zero", size), sw("t2", 4, "t1"),
         *li("t2", VSYNC), lw("t2", 0, "t2"), sw("t2", 8, "t1"), lui("t0", DATA >> 16), lw("t2", CNT & 0xffff, "t0"),
         sw("t2", 12, "t1")]
    for k in range(4):
        p += [lw("t2", s + 4 + 4 * k, "t0"), sw("t2", 0x10 + 4 * k, "t1")]
    p += [sw("v0" if exit else "zero", 0x20, "t1")]
    if "1" in deref: p += [lw("t7", s + 8, "t0")] + copy("t7", 4, 0x24)
    if "2" in deref: p += [lw("t7", s + 12, "t0"), lw("t2", 0, "t7"), sw("t2", 0x34, "t1")]
    if "3" in deref: p += [lw("t7", s + 16, "t0"), lw("t2", 0, "t7"), sw("t2", 0x38, "t1")]
    p += [lw("t6", s + 4, "t0")] + copy("t6", 0xa0, 0x40)
    p += [lw("t6", 4, "t6")]  # the player
    for off, n, at in [(0x12b0, 4, 0x2c0), (0x3d60, 8, 0x2d0), (0x3ec0, 4, 0x2f0), (0x3f90, 4, 0x300)]:
        p += [addiu("t7", "t6", off)] + copy("t7", n, at)
    p += [lw("t7", 0x54, "t6"), lw("t7", 0, "t7"), lw("t7", 0, "t7"), lbu("t2", 0x135, "t7"), sw("t2", 0x360, "t1")]
    p += li("t7", 0x423048) + copy("t7", 8, 0x310) + li("t7", 0x316600) + copy("t7", 4, 0x330)
    p += [lui("t0", DATA >> 16), lw("t6", s + 4, "t0"), lw("t7", 8, "t6"), addiu("t7", "t7", 0xf0)] + copy("t7", 8, 0x340)
    if HOOKS[tag][3] and not exit: p += li("t7", 0x424e90) + copy("t7", PATH // 4, REC)
    return p


def entry(tag, first, second):
    addr = HOOKS[tag][0]
    size = REC + (PATH if HOOKS[tag][3] else 0)
    s = save(tag) & 0xffff
    p = [lui("t0", DATA >> 16), sw("ra", s, "t0"), sw("a0", s + 4, "t0"), sw("a1", s + 8, "t0"), sw("a2", s + 12, "t0"),
         sw("a3", s + 16, "t0"), *li("t1", SCRATCH + 0x2000 * tag)] + record(tag, size, False)
    p += [*li("ra", exit_at(tag)), first, second, j(addr + 8), 0]
    return p


def exit_at(tag): return CODE + 0x1000 * tag + 0x800


def leave(tag):
    _, _, mask, path = HOOKS[tag]
    size = REC + (PATH if path else 0)
    scratch, s = SCRATCH + 0x2000 * tag, save(tag) & 0xffff
    p = [addiu("t7", "zero", 0)]  # keep
    if mask & 1:
        p += [lui("t0", DATA >> 16), lw("t2", CNT & 0xffff, "t0"), *li("t3", scratch), lw("t3", 12, "t3"),
              br(4, "t2", "t3", "d1"), 0, addiu("t7", "zero", 1), "d1"]
    if mask & 2:
        p += [br(5, "v0", "zero", "d2"), 0, addiu("t7", "zero", 1), "d2"]
    if mask & 4:
        p += [lui("t0", DATA >> 16), lw("t6", s + 4, "t0"), lw("t6", 4, "t6"), lw("t2", 0x3ec4, "t6"),
              addiu("t3", "zero", 1), br(5, "t2", "t3", "d4"), 0, addiu("t7", "zero", 1), "d4"]
    if mask & 8:
        p += [lui("t0", DATA >> 16), lw("t6", s + 4, "t0"), lbu("t2", 0x55, "t6"), *li("t3", scratch),
              lbu("t3", 0x40 + 0x55, "t3"), br(4, "t2", "t3", "d8"), 0, addiu("t7", "zero", 1), "d8"]
    p += [br(4, "t7", "zero", "done"), 0, lui("t0", DATA >> 16), lw("t1", PTR & 0xffff, "t0"), *li("t6", scratch)]
    p += copy("t6", size // 4, 0) + [addiu("t1", "t1", size)] + record(tag, REC, True)
    p += [lui("t0", DATA >> 16), addiu("t1", "t1", REC), sw("t1", PTR & 0xffff, "t0"), "done",
          lui("t0", DATA >> 16), lw("ra", s, "t0"), jr("ra"), 0]
    return p


def counter(first, second):
    return [lui("t0", DATA >> 16), *li("t1", MT), br(5, "a0", "t1", "skip"), 0,
            lw("t2", CNT & 0xffff, "t0"), br(5, "t2", "zero", "inc"), 0,
            addiu("t6", "a0", 0), *li("t1", SNAP)] + copy("t6", 0x9c8 // 4, 0) + [
            "inc", lui("t0", DATA >> 16), lw("t2", CNT & 0xffff, "t0"), addiu("t2", "t2", 1), sw("t2", CNT & 0xffff, "t0"),
            "skip", first, second, j(0x19f5c0 + 8), 0]


slot, want, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
p = Pine()  # the hooks log every frame themselves: any speed, no lock-step
p.load_state(slot)
time.sleep(0.3)
gm0 = p.read32(GM_PTR)
installed = []


def patch(at, stub, build):
    a, b = p.read32(at), p.read32(at + 4)
    words = assemble(stub, build(a, b))
    assert len(words) * 4 <= 0x800, hex(at)
    for n, w in enumerate(words): p.write32(stub + 4 * n, w)
    # the jump first: until the nop lands its delay slot runs the second instruction twice, which is harmless
    # (none of the replaced pairs writes what it reads)
    p.write32(at, j(stub))
    p.write32(at + 4, 0)
    installed.append((at, a, b))


p.write32(PTR, BUF)
p.write32(CNT, 0)
for tag in HOOKS:
    for n, w in enumerate(assemble(exit_at(tag), leave(tag))): p.write32(exit_at(tag) + 4 * n, w)
patch(0x19f5c0, CODE, counter)
for tag, (at, *_rest) in HOOKS.items():
    patch(at, CODE + 0x1000 * tag, lambda a, b, tag=tag: entry(tag, a, b))
v0 = p.read32(VSYNC)
try:
    while p.read32(VSYNC) - v0 < want and p.read32(PTR) < END - 0x40000 and p.read32(GM_PTR) == gm0:
        time.sleep(0.5)
finally:
    for at, a, b in reversed(installed):
        p.write32(at + 4, b)
        p.write32(at, a)
time.sleep(0.2)
end = p.read32(PTR)
data = p.read_block(BUF, (end - BUF + 7) & ~7)[: end - BUF]
open(out, "wb").write(b"AISV" + bytes(4) + p.read_block(SNAP, 0x9c8) + data)
print("done", end - BUF, "bytes over", p.read32(VSYNC) - v0, "frames, draws", p.read32(CNT))
