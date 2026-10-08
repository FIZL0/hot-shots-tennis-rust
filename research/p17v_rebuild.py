#!/usr/bin/env python3
"""Rebuild the court's sun-shade map in the running game and keep the frame buffer's red per pixel (P17v).
Usage: p17v_rebuild.py <slot> <out-prefix>

Loads the save state, sets the shade object's build countdown (+0x3c) to 1 so the court update rebuilds the map
the next frame, and patches the build's read-back loop to store each pixel's red byte (1280 × 896, row = game x,
column = game z) instead of thresholding it. Writes <out>.red (raw bytes) and <out>.map (the map left in RAM,
built unpatched first, so it is the game's own threshold). The save state is never written; patches are removed."""
import sys, time
sys.path.insert(0, "tools")
from pine import Pine

VSYNC, RAW = 0x1d5780, 0x1e10000
COLS, ROWS = 0x500, 0x380
LOOP = 0x33cbfc  # sltiu a3,a3,0x70 … sb a3,0(a1); 7 words after the lbu of the pixel's red
T1, T3, T4, A1, A3 = 9, 11, 12, 5, 7


def sll(rd, rt, sa): return rt << 16 | rd << 11 | sa << 6
def addu(rd, rs, rt): return rs << 21 | rt << 16 | rd << 11 | 0x21
def lui(rt, imm): return 0x0f << 26 | rt << 16 | imm & 0xffff
def ori(rt, rs, imm): return 0x0d << 26 | rs << 21 | rt << 16 | imm & 0xffff
def sb(rt, off, rs): return 0x28 << 26 | rs << 21 | rt << 16 | off & 0xffff


slot, out = int(sys.argv[1]), sys.argv[2]
p = Pine()
p.load_state(slot)
v = p.read32(VSYNC)
while p.read32(VSYNC) - v < 8:
    time.sleep(0.05)
gm = p.read32(0x422f80)
sh = p.read32(p.read32(gm + 0x84) + 0x138)
mp = p.read32(sh + 0x28)
print(f"shade {sh:#x} map {mp:#x} countdown {p.read32(sh + 0x3c)}")
orig = [p.read32(LOOP + 4 * k) for k in range(7)]


def rebuild():
    p.write32(sh + 0x3c, 1)
    t = time.monotonic()
    while p.read32(sh + 0x3c) != 0:
        if time.monotonic() - t > 20: sys.exit("countdown never ran out")
        time.sleep(0.02)
    v = p.read32(VSYNC)
    while p.read32(VSYNC) - v < 120:  # the build spans several vsyncs (read-backs)
        time.sleep(0.05)


rebuild()
m = b"".join(p.read_block(mp + o, min(0x10000, COLS * ROWS // 8 - o)) for o in range(0, COLS * ROWS // 8, 0x10000))
open(out + ".map", "wb").write(m)
# store the red byte at RAW + pixel: a1 = map byte, t1 = bit, so pixel = (a1 − map)·8 + t1
k = (RAW - mp * 8) & 0xffffffff
patch = [sll(T3, A1, 3), addu(T3, T3, T1), lui(T4, k >> 16), ori(T4, T4, k), addu(T3, T3, T4), sb(A3, 0, T3), 0]
p.pause()
for n, w in enumerate(patch):
    p.write32(LOOP + 4 * n, w)
p.resume()
try:
    rebuild()
finally:
    p.pause()
    for n, w in enumerate(orig):
        p.write32(LOOP + 4 * n, w)
    p.resume()
red = b"".join(p.read_block(RAW + o, min(0x10000, COLS * ROWS - o)) for o in range(0, COLS * ROWS, 0x10000))
open(out + ".red", "wb").write(red)
# the map is left zeroed by the patched pass: rebuild it unpatched so the game keeps its own
rebuild()
print("set bits", sum(bin(b).count("1") for b in m), "red>0x6f", sum(r > 0x6f for r in red))
