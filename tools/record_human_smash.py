#!/usr/bin/env python3
"""Record a doubles game with P1 human (slot 3/4) driven by the virtual pad, built for human smashes: the opponents'
locked ground strokes and volleys are turned into lobs (+0x3ee4 = △ (4) before the hit), and P1 presses a shot button
once per incoming ball 4 frames before it comes down through 2.8 m, steering there with the stick, cycling ✕, ○, △ by smash (the button moves on once a
smash locks; smash kinds 0, 0, 1).
With nothing to hit for 2 s it presses ✕ (point-over skip; serving, every 40 frames: toss, hit). Needs tools/vpad.py serve (pcsx2-hst.sh
starts it for HST_PCSX2 copies). Usage: record_human_smash.py <slot> <out.bin> <frames>. Same sample layout as
record_p2m2.py (`hst_sim::replay::frames_live`). A PCSX2 copy runs at 1x (tools/pine.py; HST_LOCKSTEP=1: every frame, slowly); the user's own PCSX2: run it slowed (NominalScalar 0.25)."""
import os, struct, sys, time
from pine import Pine
from vpad import FIFO

VSYNC, GM_PTR = 0x1d5780, 0x422f80
PAD, GLOBALS, RALLY = (0x2efb00, 0x90), (0x422f80, 0x180), (0x3165f0, 0x50)
PLAYER = ((0x1380, 0x200), (0x3c00, 0x400))
TRIANGLE = 4

def s32(v): return struct.unpack("<i", struct.pack("<I", v))[0]
def f32(v): return struct.unpack("<f", struct.pack("<I", v))[0]

pad = os.open(FIFO, os.O_RDWR)  # held open so each command is a line, not a reopen
def send(*cmds): os.write(pad, ("\n".join(cmds) + "\n").encode())
releases = []  # (sample, button): a press is held 3 frames, counted in frames so it lands alike at any speed
def press(btn): send(f"down {btn}"); releases.append((n + 3, btn))

p, out, want = Pine(step=True), open(sys.argv[2], "wb"), int(sys.argv[3])
send("release")
p.load_state(int(sys.argv[1]))
time.sleep(2)  # the load lands asynchronously; pointers read before it are stale
gm = p.read32(GM_PTR)
players = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
r = [PAD, GLOBALS, (gm, 0x100), (p.read32(gm + 0x98), 0x290)]
r += [(pl + o, n) for pl in players for o, n in PLAYER] + [(p.read32(gm + 0x88), 0x290), RALLY]
ball = p.read32(gm + 0x88)
last, n, lobs, presses, idle, pressed_for, prev, held = p.read32(VSYNC), 0, 0, 0, 0, None, None, (0, 0)
smashes, smashed_for = 0, None
while n < want:
    v = p.next_frame(last)
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    a = p.read_regions(r)
    while True:
        b = p.read_regions(r)
        if a == b and p.read32(VSYNC) == v: break
        a = b
    out.write(struct.pack("<I", v) + a)
    n += 1
    while releases and releases[0][0] <= n: send(f"up {releases.pop(0)[1]}")
    for k in (1, 3):  # the opponents lob
        pl = players[k]
        if p.read8(pl + 0x3ec1) in (1, 2) and s32(p.read32(pl + 0x3ec4)) >= 0 and p.read32(pl + 0x3ee4) != TRIANGLE:
            p.write32(pl + 0x3ee4, TRIANGLE)
            lobs += 1
    me = players[0]
    pos, vel = [f32(p.read32(ball + 0x120 + 4 * j)) for j in range(3)], [f32(p.read32(ball + 0x130 + 4 * j)) for j in range(3)]
    acc = [x - y for x, y in zip(vel, prev)] if prev else [0, 0.01, 0]
    prev = vel
    mx, mz = f32(p.read32(me + 0x3d70)), f32(p.read32(me + 0x3d78))
    hit = tuple(p.read32(ball + o) for o in (0x70, 0x74, 0x78))  # a new hit point marks a new ball
    idle += 1
    # where the ball next comes down through smash height (2.8 m) on our half, stepping its current acceleration
    q, u, eta = list(pos), list(vel), None
    if vel[2] * mz > 0 and acc[1] > 0:
        for k in range(1, 90):
            u = [x + y for x, y in zip(u, acc)]
            q = [x + y for x, y in zip(q, u)]
            if q[2] * mz > 0 and u[1] > 0 and -q[1] <= 2.8:
                eta = k
                break
    if eta is not None:
        dx, dz = q[0] - mx, q[2] - mz
        d = (dx * dx + dz * dz) ** 0.5
        s = 1 if mz > 0 else -1  # the camera sits behind P1: on the near half stick right is −x, down +z
        stick = (round(-s * dx / d, 1), round(s * dz / d, 1)) if d > 0.3 else (0, 0)
    else:
        stick = (0, 0)
    if stick != held:
        held = stick
        send(f"stick l {stick[0]} {stick[1]}")
    if hit != pressed_for and eta is not None and eta <= 4:
        pressed_for, idle = hit, 0
        btn = ("cross", "circle", "triangle")[smashes % 3]
        presses += 1
        press(btn)
        print(f"vsync {v}: {btn}, ball {eta} frames from smash height, {d:.2f} m off", flush=True)
    elif p.read8(me + 0x3ec1) == 4 and s32(p.read32(me + 0x3ec4)) >= 0 and smashed_for != pressed_for:
        smashed_for = pressed_for  # a smash locked: the next button
        smashes += 1
        print(f"vsync {v}: smash {smashes}", flush=True)
    elif idle > (40 if p.read8(me + 0x3ec1) == 5 else 120):  # serving: toss, then hit
        idle = 0
        press("cross")
    if n % 1200 == 0: print(n, flush=True)
send("release")
print("done", n, "lobs", lobs, "presses", presses, "smashes", smashes)
