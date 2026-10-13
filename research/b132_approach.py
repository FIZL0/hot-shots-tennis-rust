#!/usr/bin/env python3
"""B132: what a human press does when the ball is out of reach (the auto-approach, then the one contact search).
Loads slot 3/4 (P1 human), steers P1 to stand <side> m beside where each incoming ball crosses its line and presses
✕ when the game's predicted path is <lead> frames from P1's contact depth (sides and leads cycle, so early, late and
far presses). From 2 frames before each press until 6 after the lock or the whiff it logs every frame: P1's state
mode +0x3fa5, approach flag +0x3f24 and count +0x3f28, its direction +0x3f30, branch +0x3ec1, countdown +0x3ec4,
position +0x3d70, run count +0x3dfc, and the predictor's index (+0x1400 → +0x58) and the ball. The stick is let go
on the press frame (the approach steers), so every moved frame is the approach's or the stroke's.
Usage: b132_approach.py <slot> <presses> <max frames> [sides] [leads] > log."""
import os, struct, sys, time
sys.path.insert(0, "tools")
from pine import Pine
from vpad import FIFO

VSYNC, GM_PTR = 0x1d5780, 0x422f80
slot, want, cap = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
SIDES = [float(x) for x in (sys.argv[4] if len(sys.argv) > 4 else "2.0,2.6,1.8,3.0").split(",")]
LEADS = [int(x) for x in (sys.argv[5] if len(sys.argv) > 5 else "14,8,20,11,5").split(",")]
AHEAD = 0.8
s32 = lambda u: struct.unpack("<i", struct.pack("<I", u))[0]
f32 = lambda u: struct.unpack("<f", struct.pack("<I", u))[0]
pad = os.open(FIFO, os.O_RDWR)
send = lambda *c: os.write(pad, ("\n".join(c) + "\n").encode())

p = Pine()
send("release")
p.load_state(slot)
time.sleep(2)
gm = p.read32(GM_PTR)
me, ball = p.read32(gm + 0xa8), p.read32(gm + 0x88)

def contact_frame(mz, s):
    rec = p.read32(me + 0x1400)
    buf, ln, now = p.read32(rec + 0x50), s32(p.read32(rec + 0x54)), s32(p.read32(rec + 0x58))
    m = min(ln - now, 40)
    if m <= 0: return None
    b, zc = p.read_block(buf + 0x30 * now, 0x30 * m), mz - s * AHEAD
    z = lambda k: struct.unpack_from("<f", b, 0x30 * k + 8)[0]
    ks = [k for k in range(m) if struct.unpack_from("<i", b, 0x30 * k + 0x20)[0] <= 1 and z(k) * s > 0.5]
    return min(ks, key=lambda k: abs(z(k) - zc)) if ks else None

last, n, presses, held, watch, after, pressed_for, idle = p.read32(VSYNC), 0, 0, None, None, 0, None, 0
hist = []
while presses <= want and n < cap:
    v = p.next_frame(last)
    if v != last + 1: print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last, n = v, n + 1
    a = p.read_block(me + 0x3d70, 0x10)
    mx, mz = struct.unpack_from("<f", a, 0)[0], struct.unpack_from("<f", a, 8)[0]
    st = p.read_block(me + 0x3ec0, 0xf0)
    mode, flag = p.read8(me + 0x3fa5), p.read8(me + 0x3f24)
    br, cd = st[1], s32(struct.unpack_from("<I", st, 4)[0])
    cnt, run = s32(p.read32(me + 0x3f28)), s32(p.read32(me + 0x3dfc))
    d = struct.unpack("<4f", p.read_block(me + 0x3f30, 0x10))
    rec = p.read32(me + 0x1400)
    now = s32(p.read32(rec + 0x58))
    bp = [f32(p.read32(ball + 0x120 + 4 * j)) for j in range(3)]
    bv = [f32(p.read32(ball + 0x130 + 4 * j)) for j in range(3)]
    line = (f"{v} mode {mode} appr {flag}/{cnt} dir ({d[0]:.4f},{d[2]:.4f}) br {br} cd {cd} pos ({mx:.4f},{mz:.4f}) "
            f"run {run} pred {now} ball ({bp[0]:.3f},{bp[1]:.3f},{bp[2]:.3f}) held {held}")
    hist = (hist + [line])[-3:]
    if watch is not None:
        print(line, flush=True)
        if br in (1, 2, 4) and cd >= 0 or mode not in (1, 2) or n - watch > 45:
            after += 1
            if after > 6: watch, after = None, 0; print("--", flush=True)
    s = 1 if mz > 0 else -1
    stick = (0, 0)
    hit = tuple(p.read32(ball + o) for o in (0x70, 0x74, 0x78))
    if watch is None and bv[2] * mz > 0 and p.read8(gm + 0x55) == 3 and hit != pressed_for and cd < 0:
        side = SIDES[presses % len(SIDES)]
        dx = bp[0] + bv[0] * (mz - bp[2]) / bv[2] - mx
        dx -= side if dx > 0 else -side
        stick = (round(-s * max(-1, min(1, 2 * dx)), 1), 0) if abs(dx) > 0.15 else (0, 0)
        k = contact_frame(mz, s)
        if k is not None and k <= LEADS[presses % len(LEADS)]:
            pressed_for, idle, stick = hit, 0, (0, 0)
            send("stick l 0 0", "down cross")
            held = (0, 0)
            print("\n".join(hist[:-1]), flush=True)
            print(f"PRESS {presses} at {v}: side {side} lead {LEADS[presses % len(LEADS)]} contact in {k} (stick let go)", flush=True)
            print(line, flush=True)
            presses, watch = presses + 1, n
            continue
    if watch is not None and n - watch == 3: send("up cross")
    if watch is None and stick != held:
        held = stick
        send(f"stick l {stick[0]} {stick[1]}")
    idle += 1
    if watch is None and idle > 120 and p.read8(gm + 0x55) != 3:
        idle = 0
        send("down cross"); time.sleep(0.05); send("up cross")
send("release")
print("done", n, presses)
