#!/usr/bin/env python3
"""Record P1's (human, slot 3/4) rally aims, driven by the virtual pad: P1 presses ✕, ○, △ in turn as each ball comes
toward it (steering sideways at the ball until then), and from the swing's lock to its contact holds the next aim
stick of a cycle (corners first, then edges, centre and part tilts). Writes a pair of samples per aim: the frame
before P1's +0x3e90 changes and the frame it does, each a `frames_live` sample followed by P1's +0x12b0..+0x1320
(end, character, TParam aim values). Needs tools/vpad.py serve (pcsx2-hst.sh starts it for HST_PCSX2 copies).
Usage: record_aim.py <slot> <out.bin> <aims> [max frames]. Run PCSX2 slowed down (NominalScalar 0.5)."""
import os, struct, sys, time
from pine import Pine
from vpad import FIFO

VSYNC, GM_PTR = 0x1d5780, 0x422f80
PAD, GLOBALS, RALLY = (0x2efb00, 0x90), (0x422f80, 0x180), (0x3165f0, 0x50)
PLAYER = ((0x1380, 0x200), (0x3c00, 0x400))
STICKS = [(1, -1), (-1, -1), (1, 1), (-1, 1), (0, -1), (1, 0), (-1, 0), (0, 1), (0, 0), (0.6, -0.8), (-0.4, -1)]  # 11: coprime to the 3 buttons
BUTTONS = ("cross", "circle", "triangle")

def s32(v): return struct.unpack("<i", struct.pack("<I", v))[0]
def f32(v): return struct.unpack("<f", struct.pack("<I", v))[0]

pad = os.open(FIFO, os.O_RDWR)
def send(*cmds): os.write(pad, ("\n".join(cmds) + "\n").encode())

p, out, want = Pine(), open(sys.argv[2], "wb"), int(sys.argv[3])
cap = int(sys.argv[4]) if len(sys.argv) > 4 else 30000
send("release")
p.load_state(int(sys.argv[1]))
time.sleep(2)
gm = p.read32(GM_PTR)
players = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
me, ball = players[0], p.read32(gm + 0x88)
r = [PAD, GLOBALS, (gm, 0x100), (p.read32(gm + 0x98), 0x290)]
r += [(pl + o, n) for pl in players for o, n in PLAYER] + [(ball, 0x290), RALLY, (me + 0x12b0, 0x70)]
AIM = 4 + 0x90 + 0x180 + 0x100 + 0x290 + 0x200 + 0x3e90 - 0x3c00  # P1's +0x3e90 in a sample
BRANCH = AIM + 0x3ec1 - 0x3e90
last, n, aims, prev_sample, pressed_for, held, idle, presses = p.read32(VSYNC), 0, 0, None, None, (0, 0), 0, 0
while aims < want and n < cap:
    v = p.read32(VSYNC)
    if v == last:
        continue
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    time.sleep(0.004)
    a = p.read_regions(r)
    while True:
        b = p.read_regions(r)
        if a == b and p.read32(VSYNC) == v: break
        a = b
    sample = struct.pack("<I", v) + a
    n += 1
    if prev_sample and prev_sample[BRANCH] in (1, 2, 3, 4) and sample[AIM:AIM + 16] != prev_sample[AIM:AIM + 16] and last - struct.unpack("<I", prev_sample[:4])[0] == 1:
        out.write(prev_sample + sample)
        out.flush()
        aims += 1
        t = struct.unpack("<4f", sample[AIM:AIM + 16])
        print(f"vsync {v}: aim {aims} branch {prev_sample[BRANCH]} kind {sample[BRANCH + 0x23]} stick {held} -> ({t[0]:.3f}, {t[2]:.3f})", flush=True)
    prev_sample = sample
    branch, count = p.read8(me + 0x3ec1), s32(p.read32(me + 0x3ec4))
    locked = branch in (1, 2, 3, 4) and count >= 0
    pos, vel = [f32(p.read32(ball + 0x120 + 4 * j)) for j in range(3)], [f32(p.read32(ball + 0x130 + 4 * j)) for j in range(3)]
    mx, mz = f32(p.read32(me + 0x3d70)), f32(p.read32(me + 0x3d78))
    hit = tuple(p.read32(ball + o) for o in (0x70, 0x74, 0x78))
    s = 1 if mz > 0 else -1  # the camera sits behind P1: on the +z half stick right is −x
    idle += 1
    if locked:
        stick = STICKS[presses % len(STICKS)]
    elif vel[2] * mz > 0 and hit != pressed_for:
        dx = pos[0] - mx
        stick = (round(-s * max(-1, min(1, dx)), 1), 0) if abs(dx) > 0.6 else (0, 0)
        if abs(pos[2] - mz) < 11:
            pressed_for, idle = hit, 0
            presses += 1
            send(f"press {BUTTONS[presses % 3]} 80")
    else:
        stick = (0, 0)
    if stick != held:
        held = stick
        send(f"stick l {stick[0]} {stick[1]}")
    if not locked and idle > (40 if branch == 5 else 120):
        idle = 0
        send("press cross 80")
send("release")
print("done", n, "frames", aims, "aims", presses, "presses")
