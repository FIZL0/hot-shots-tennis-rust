#!/usr/bin/env python3
"""P7e: record dives by a chosen character as P1. Run under tools/pcsx2.sh (HST_PCSX2=N copy, NominalScalar 0.5).
  p7e_dive_record.py <char> <out.bin> [frames=3600] [--slot N]
tools/pick.py's match (P1 = <char>, saved to scratch 8; --slot skips the pick and loads slot N), then one PINE
connection both samples every frame (record_p2m2.py's format: frames_live) and drives P1 over the virtual pad: it
serves (○ every 1.5 s while serving), and when the opponents' ball comes in it runs toward where the ball crosses
P1's depth and presses ✕ while that is still 8–13 frames off and 2–4.5 m away (until 13 frames off it drifts away from it): a press that finds no stroke
while running dives. The stick's axis signs are learned from P1's run velocity."""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "../tools"))
from pine import Pine
from pick import pick, vpad

VSYNC, GM_PTR = 0x1d5780, 0x422f80
PAD, GLOBALS, GM, BALL, RALLY = (0x2efb00, 0x90), (0x422f80, 0x180), 0x100, 0x290, (0x3165f0, 0x50)
PLAYER = ((0x1380, 0x200), (0x3c00, 0x400))
# offsets in a sample (tests: hst_sim::replay)
P0 = 4 + 0x90 + 0x180 + 0x100 + 0x290
LIVE = P0 + 4 * 0x600
T = os.path.join(os.path.dirname(__file__), "../tools")

def regions(p):
    gm = p.read32(GM_PTR)
    r = [PAD, GLOBALS, (gm, GM), (p.read32(gm + 0x98), BALL)]
    for i in range(4):
        pl = p.read32(gm + 0xa8 + 4 * i)
        r += [(pl + o, n) for o, n in PLAYER]
    return gm, r + [(p.read32(gm + 0x88), BALL), RALLY]

def pf(a, o): return struct.unpack_from("<f", a, P0 + 0x200 + o - 0x3c00)[0]
def pb(a, o): return a[P0 + 0x200 + o - 0x3c00]

def send(*c): subprocess.run([f"{T}/vpad.py", "send", *c], check=True)

ch, out = int(sys.argv[1]), sys.argv[2]
frames = int(sys.argv[3]) if len(sys.argv) > 3 and sys.argv[3].isdigit() else 3600
p = Pine()
if "--slot" in sys.argv:
    slot = int(sys.argv[sys.argv.index("--slot") + 1])
else:
    slot = 8
    # copy 2's pad reaches the game with ○/✕ swapped (vpad "circle" cancels the pick): pick through "cross"
    import pick as pk
    pk.vpad = lambda *c, v=pk.vpad: v(*[l.replace("circle", "cross") for l in c])
    print(pick(p, ch, slot)[1], flush=True)
last = p.read32(VSYNC)
p.load_state(slot)
time.sleep(0.3)
f = open(out, "wb")
n, sx, sz, stick, serve_t, press_t, wrong, dives, was_diving = 0, 1.0, -1.0, (0.0, 0.0), -999, -999, [0, 0], 0, False
last = p.read32(VSYNC)
while n < frames:
    v = p.read32(VSYNC)
    if v == last:
        continue
    last = v
    time.sleep(0.004)
    gm, r = regions(p)
    a = p.settle(r, v)
    if p.read32(GM_PTR) != gm: sys.exit(f"match object gone at vsync {v}, {n} samples")
    if a is None:
        print(f"missed frame {v}", flush=True)
        continue
    a = struct.pack("<I", v) + a  # offsets below are the recording's (vsync first)
    f.write(a)
    n += 1
    phase = a[4 + 0x90 + 0x180 + 0x55]
    pos = (pf(a, 0x3d70), pf(a, 0x3d78))
    vel = (pf(a, 0x3e00), pf(a, 0x3e08))
    if pb(a, 0x3f58) == 1 and not was_diving:
        dives += 1
        print(f"dive at vsync {v} (sample {n - 1}) kind {struct.unpack_from('<i', a, P0 + 0x200 + 0x3ec4 - 0x3c00)[0]}", flush=True)
    was_diving = pb(a, 0x3f58) == 1
    want = (0.0, 0.0)
    if pb(a, 0x3fa4) == 1 and pb(a, 0x3fa6) <= 2:
        if n - serve_t > 45:
            send("press circle 80")
            serve_t = n
    elif phase == 3:
        b = struct.unpack_from("<3f", a, LIVE + 0xe0)
        bv = struct.unpack_from("<3f", a, LIVE + 0x130)
        side = 1.0 if pos[1] > 0 else -1.0
        if bv[2] * side > 0.01:
            t = (pos[1] - b[2]) / bv[2]
            q = (b[0] + bv[0] * t, pos[1] - side * 0.3)
            dx, dz = q[0] - pos[0], q[1] - pos[1]
            d = (dx * dx + dz * dz) ** 0.5
            # drift away sideways until the ball is 16 frames off (≤ 3 m), so that the press finds it out of reach
            if 16 < t <= 30 and d < 3.0:
                want = (-1.0 if dx > 0 else 1.0, 0.0)
            elif 0 < t <= 16 and d > 0.3:
                want = (dx / d, dz / d)
                if 6 <= t <= 13 and 1.8 <= d <= 4.5 and pb(a, 0x3fa5) == 1 and n - press_t > 30:  # running
                    send("press cross 80")
                    press_t = n
    # learn the stick's signs from the run velocity
    for k, (w, vv) in enumerate(zip(want, vel)):
        if abs(w) > 0.6 and abs(vv) > 0.03:
            wrong[k] = wrong[k] + 1 if w * vv < 0 else 0
            if wrong[k] > 15:  # turning round lags the velocity a few frames
                if k == 0: sx = -sx
                else: sz = -sz
                wrong[k] = 0
                print("flipped axis", k, flush=True)
    s = (round(want[0] * sx, 2), round(want[1] * sz, 2))
    if s != stick:
        send("release") if s == (0.0, 0.0) else send(f"stick l {s[0]} {s[1]}")
        stick = s
send("release")
print("done", n, "dives", dives)
