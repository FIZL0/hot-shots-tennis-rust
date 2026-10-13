#!/usr/bin/env python3
"""B59: how long a whiff holds a player that keeps the stick held, and whether the run (dash) counter carries on.
Loads slot 3/4 (P1 human), serves with ✕ when idle. Two cases, alternating:
  A (run): with the ball going away from P1 (its team hit last), P1 runs sideways with the stick held for <run>
    frames, then presses ✕ with the stick still held (a swing at nothing, countdown −2), and keeps the stick held.
  B (stand): with the opponents' ball just struck and far away, P1 stands, presses ✕ (nothing to reach: −3 whiff
    with the miss motion) and holds the stick from the next frame.
From the first stick frame (A) or the press (B) for 70 frames it logs: P1's state +0x3fa4/+0x3fa5, countdown
+0x3ec4, frames since the pose +0x3f00, run count +0x3dfc, motion +0x3df0, run velocity +0x3e00, position +0x3d70.
Usage: b59_whiff_dash.py <slot> <trials> <max frames> [run frames] > log."""
import os, struct, sys, time
sys.path.insert(0, "tools")
from pine import Pine
from vpad import FIFO

VSYNC, GM_PTR, LAST_HITTER = 0x1d5780, 0x422f80, 0x423058
slot, want, cap = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
RUN = int(sys.argv[4]) if len(sys.argv) > 4 else 30
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

last, n, trials, case, t, idle, done_for = p.read32(VSYNC), 0, 0, "A", None, 0, None
while trials < want and n < cap:
    v = p.next_frame(last)
    if v != last + 1: print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last, n = v, n + 1
    phase, lh = p.read8(gm + 0x55), s32(p.read32(LAST_HITTER))
    a = p.read_block(me + 0x3d70, 0x10)
    mx, mz = struct.unpack_from("<f", a, 0)[0], struct.unpack_from("<f", a, 8)[0]
    bp = [f32(p.read32(ball + 0x120 + 4 * j)) for j in range(3)]
    hit = tuple(p.read32(ball + o) for o in (0x70, 0x74, 0x78))
    if t is not None:
        st, mode, cd = p.read8(me + 0x3fa4), p.read8(me + 0x3fa5), s32(p.read32(me + 0x3ec4))
        since, run, mo = s32(p.read32(me + 0x3f00)), s32(p.read32(me + 0x3dfc)), s32(p.read32(me + 0x3df0))
        vel = struct.unpack_from("<3f", p.read_block(me + 0x3e00, 0x10))
        print(f"{case} t {t:3d} v {v} st {st} mode {mode} cd {cd} since {since} run {run} motion {mo:#x} "
              f"vel ({vel[0]:.5f},{vel[2]:.5f}) pos ({mx:.4f},{mz:.4f}) ball ({bp[0]:.2f},{bp[1]:.2f},{bp[2]:.2f}) phase {phase}",
              flush=True)
        t += 1
        if case == "A" and t == RUN:
            send("down cross"); print(f"PRESS A at {v + 1}", flush=True)
        if case == "A" and t == RUN + 3: send("up cross")
        if case == "B" and t == 1: send("up cross", "stick l 1 0")
        if t >= (RUN if case == "A" else 0) + 70 or phase != 3:
            send("release", "stick l 0 0"); print("--", flush=True)
            t, trials, done_for = None, trials + 1, hit
            case = "B" if case == "A" else "A"
        continue
    if phase == 3 and hit != done_for:
        side = 1 if mx * (1 if mz > 0 else -1) < 0 else -1  # toward the middle
        if case == "A" and lh >= 0 and lh & 1 == 0 and bp[2] * mz < 0:
            send(f"stick l {side} 0"); t = 0; print(f"RUN A from {v + 1} (stick {side})", flush=True)
        elif case == "B" and lh >= 0 and lh & 1 == 1 and bp[2] * mz < 0 and abs(bp[2]) > 9:
            send("stick l 0 0", "down cross"); t = 0; print(f"PRESS B at {v + 1}", flush=True)
        continue
    idle += 1
    if idle > 120 and phase != 3:
        idle = 0
        send("down cross"); time.sleep(0.05); send("up cross")
send("release")
print("done", n, trials)
