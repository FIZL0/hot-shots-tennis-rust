#!/usr/bin/env python3
"""Record P1's (human, slot 3/4) rally aims, driven by the virtual pad: P1 runs to stand a reach (AIM_REACH, 1 m)
beside where each ball crosses its line (lined up with it nothing swings; the swing locks by itself, so its timing
offset is the game's), and from the lock to the contact holds the next aim stick of a cycle (corners first, then
edges, centre and part tilts). Every other aim is made a sweet-spot hit (+0x3fa0 = 0 during the lock); the opponents
slice two shots in three, half of those sweet (the aim's incoming cases). AIM_SINGLES=1: the player count (0x422fa4)
reads 2 over P1's contact, so the aim takes its singles width in a doubles save (no singles save exists).
Writes a pair of samples per aim: the frame before P1's +0x3e90 changes and the frame it does, each a `frames_live`
sample followed by P1's +0x12b0..+0x1320 (end, character, TParam aim values) and the last shot's record
(P1 +0x1400 → +0x1b0..+0x1c0). AIM_DEBUG=1 logs the approach. Needs tools/vpad.py serve (pcsx2-hst.sh starts it).
Usage: record_aim.py <slot> <out.bin> <aims> [max frames]. A PCSX2 copy runs at 1x (tools/pine.py; HST_LOCKSTEP=1: every frame, slowly); the user's own PCSX2: run it slowed (NominalScalar 0.25)."""
import os, struct, sys, time
from pine import Pine
from vpad import FIFO

VSYNC, GM_PTR = 0x1d5780, 0x422f80
PAD, GLOBALS, RALLY = (0x2efb00, 0x90), (0x422f80, 0x180), (0x3165f0, 0x50)
PLAYER = ((0x1380, 0x200), (0x3c00, 0x400))
STICKS = [(1, -1), (-1, -1), (1, 1), (-1, 1), (0, -1), (1, 0), (-1, 0), (0, 1), (0, 0), (0.6, -0.8), (-0.4, -1)]  # 11: coprime to the 3 buttons
BUTTONS = ("cross", "circle", "triangle")
SINGLES, COUNT = os.environ.get("AIM_SINGLES") == "1", 0x422fa4  # player count
SLICE, theirs = 2, {}  # ○'s +0x3ee4 code
REACH = float(os.environ.get("AIM_REACH", "1.0"))

def s32(v): return struct.unpack("<i", struct.pack("<I", v))[0]
def f32(v): return struct.unpack("<f", struct.pack("<I", v))[0]

pad = os.open(FIFO, os.O_RDWR)
def send(*cmds): os.write(pad, ("\n".join(cmds) + "\n").encode())
releases = []  # (sample, button): a press is held 3 frames, counted in frames so it lands alike at any speed
def press(btn): send(f"down {btn}"); releases.append((n + 3, btn))

p, out, want = Pine(step=True), open(sys.argv[2], "wb"), int(sys.argv[3])
cap = int(sys.argv[4]) if len(sys.argv) > 4 else 30000
send("release")
p.load_state(int(sys.argv[1]))
time.sleep(2)
gm = p.read32(GM_PTR)
players = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
me, ball = players[0], p.read32(gm + 0x88)
r = [PAD, GLOBALS, (gm, 0x100), (p.read32(gm + 0x98), 0x290)]
r += [(pl + o, n) for pl in players for o, n in PLAYER] + [(ball, 0x290), RALLY, (me + 0x12b0, 0x70), (p.read32(me + 0x1400) + 0x1b0, 0x10)]  # + the last shot's record
AIM = 4 + 0x90 + 0x180 + 0x100 + 0x290 + 0x200 + 0x3e90 - 0x3c00  # P1's +0x3e90 in a sample
BRANCH = AIM + 0x3ec1 - 0x3e90
last, n, aims, prev_sample, pressed_for, held, idle, presses = p.read32(VSYNC), 0, 0, None, None, (0, 0), 0, 0
while aims < want and n < cap:
    v = p.next_frame(last)
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    a = p.read_regions(r)
    while True:
        b = p.read_regions(r)
        if a == b and p.read32(VSYNC) == v: break
        a = b
    sample = struct.pack("<I", v) + a
    n += 1
    while releases and releases[0][0] <= n: send(f"up {releases.pop(0)[1]}")
    if prev_sample and prev_sample[BRANCH] in (1, 2, 3, 4) and sample[AIM:AIM + 16] != prev_sample[AIM:AIM + 16] and last - struct.unpack("<I", prev_sample[:4])[0] == 1:
        out.write(prev_sample + sample)
        out.flush()
        aims += 1
        t = struct.unpack("<4f", sample[AIM:AIM + 16])
        print(f"vsync {v}: aim {aims} branch {prev_sample[BRANCH]} kind {sample[BRANCH + 0x23]} stick {held} offset {s32(struct.unpack_from('<I', sample, AIM + 0x110)[0])} -> ({t[0]:.3f}, {t[2]:.3f})", flush=True)
    if os.environ.get("AIM_DEBUG") and prev_sample and (prev_sample[BRANCH] != sample[BRANCH] or sample[AIM:AIM + 16] != prev_sample[AIM:AIM + 16]):
        print(f"vsync {v}: branch {prev_sample[BRANCH]}->{sample[BRANCH]} aim {struct.unpack('<4f', sample[AIM:AIM + 16])}", flush=True)
    prev_sample = sample
    for k in (1, 3):  # the opponents slice two shots in three, half of them sweet: the aim's incoming cases
        pl = players[k]
        if p.read8(pl + 0x3ec1) in (1, 2) and s32(p.read32(pl + 0x3ec4)) >= 1:
            m = theirs.setdefault(k, len(theirs))  # this swing's number
            if m % 3 and p.read32(pl + 0x3ee4) != SLICE:
                p.write32(pl + 0x3ee4, SLICE)
            if m % 2 and p.read32(pl + 0x3fa0) != 0:
                p.write32(pl + 0x3fa0, 0)
        elif k in theirs:
            theirs[len(theirs) + 100] = theirs.pop(k)  # done: keep its number counted
    branch, count = p.read8(me + 0x3ec1), s32(p.read32(me + 0x3ec4))
    locked = branch in (1, 2, 3, 4) and count >= 0
    pos, vel = [f32(p.read32(ball + 0x120 + 4 * j)) for j in range(3)], [f32(p.read32(ball + 0x130 + 4 * j)) for j in range(3)]
    mx, mz = f32(p.read32(me + 0x3d70)), f32(p.read32(me + 0x3d78))
    hit = tuple(p.read32(ball + o) for o in (0x70, 0x74, 0x78))
    if os.environ.get("AIM_DEBUG") and n % 8 == 0 and vel[2] * mz > 0:
        print(f"  {v}: b{branch} c{count} ball ({pos[0]:.2f},{pos[1]:.2f},{pos[2]:.2f}) me ({mx:.2f},{mz:.2f}) held {held} eta {(mz - pos[2]) / vel[2]:.1f} p{presses}", flush=True)
    s = 1 if mz > 0 else -1  # the camera sits behind P1: on the +z half stick right is −x
    idle += 1
    if locked:
        stick = STICKS[presses % len(STICKS)]
        if SINGLES and count <= 2 and p.read32(COUNT) != 2:
            p.write32(COUNT, 2)  # the aim's singles width, in this doubles save: just over P1's contact
        if aims % 2 == 0 and count >= 1 and s32(p.read32(me + 0x3fa0)) != 0:
            p.write32(me + 0x3fa0, 0)  # every other aim a sweet-spot hit (the swing's timing is the auto-lock's, not the press's)
    elif vel[2] * mz > 0:
        dx = pos[0] + vel[0] * (mz - pos[2]) / vel[2] - mx  # where it crosses P1's line, from P1
        dx -= REACH if dx > 0 else -REACH  # stand a reach off it, on the nearer side (lined up, nothing swings)
        stick = (round(-s * max(-1, min(1, 2 * dx)), 1), 0) if abs(dx) > 0.15 else (0, 0)
        if (mz - pos[2]) / vel[2] < int(os.environ.get("AIM_LEAD", "6")) and hit != pressed_for:  # frames to P1's line
            pressed_for, idle = hit, 0
            presses += 1
            press(BUTTONS[presses % 3])
    else:
        stick = (0, 0)
    if SINGLES and not locked and p.read32(COUNT) != 4:
        p.write32(COUNT, 4)
    if stick != held:
        held = stick
        send(f"stick l {stick[0]} {stick[1]}")
    if not locked and idle > (40 if branch == 5 else 120):
        idle = 0
        press("cross")
if SINGLES: p.write32(COUNT, 4)
send("release")
print("done", n, "frames", aims, "aims", presses, "presses")
