#!/usr/bin/env python3
"""Capture game state every frame while PCSX2 plays an input recording (Tools → Input Recording → Play).
Usage: record_p2m2.py <state.p2s | slot> <out.bin> <frames>. With a .p2s: start it first, then start playback; it
arms on that state's load (vsync counter jumps). With a slot number it loads the state itself (bot games: slot 5).
Writes `frames` samples. Run PCSX2 slowed down ([Framerate] NominalScalar =
0.5 in PCSX2.ini; 0.5 captured new_recording gap-free, drop to 0.25 if it reports missed frames) so each frame's emulation burst ends well before the next vsync: a sample is taken only when two
reads in a row are identical, so it is never torn mid-frame.
Sample = u32 vsync counter + the regions in REGIONS order (fixed size; layout in README of the P0 journal), then
the live ball *(gm+0x88) and the rally block 0x3165f0 (not in round1.bin, which predates them)."""
import struct, sys, time, zipfile
from pine import Pine

VSYNC, GM_PTR = 0x1d5780, 0x422f80
PAD = (0x2efb00, 0x90)                  # pad manager: decoded state port0 at +0x30, port1 at +0x78 (18 bytes each)
GLOBALS = (0x422f80, 0x180)             # gm pointer, player count 0x422fa4, ...
GM, BALL = 0x100, 0x290
PLAYER = ((0x1380, 0x200), (0x3c00, 0x400))   # reach/params, positions/contact state
RALLY = (0x3165f0, 0x50)                # judge inputs, faults, let, tiebreak, deuce/advantage

def regions(p):
    gm = p.read32(GM_PTR)
    r = [PAD, GLOBALS, (gm, GM), (p.read32(gm + 0x98), BALL)]
    for i in range(4):
        pl = p.read32(gm + 0xa8 + 4 * i)
        r += [(pl + o, n) for o, n in PLAYER]
    return gm, r + [(p.read32(gm + 0x88), BALL), RALLY]

p, out, want = Pine(), open(sys.argv[2], "wb"), int(sys.argv[3])
if sys.argv[1].isdigit():
    last = p.read32(VSYNC)
    p.load_state(int(sys.argv[1]))
    time.sleep(0.3)
    start = p.read32(VSYNC)
else:
    start = struct.unpack_from("<I", zipfile.ZipFile(sys.argv[1]).read("eeMemory.bin"), VSYNC)[0]
    last = p.read32(VSYNC)
print("waiting for vsync", start, flush=True)
n, armed = 0, False
while n < want:
    v = p.read32(VSYNC)
    if v == last:
        continue
    if not armed:
        armed = abs(v - last) > 2 and start <= v <= start + 2
        if not armed:
            last = v
            continue
        print("armed at vsync", v, flush=True)
    elif v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    time.sleep(0.004)
    gm, r = regions(p)
    a = p.read_regions(r)
    while True:
        b = p.read_regions(r)
        if a == b and p.read32(VSYNC) == v and p.read32(GM_PTR) == gm: break
        if p.read32(GM_PTR) != gm: sys.exit(f"match object gone at vsync {v} (match over), {n} samples")
        a = b
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 600 == 0: print(n, flush=True)
print("done", n)
