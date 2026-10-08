#!/usr/bin/env python3
"""Record P1's (human, slot 3/4) rally aims, driven by the virtual pad: P1 runs to stand a reach (AIM_REACH, 1 m)
beside where each ball crosses its line (lined up with it nothing swings) and presses the next button AIM_LEAD frames
before the game's predicted ball reaches the contact depth, so the timing offset is the press's (sweet and off-sweet hits by timing alone; a
press locks no swing until the ball is in reach, so a stray early press gives the reach's frame instead: the idle ✕
that skips past points is never sent mid-rally). From the lock to the contact it holds the next aim stick of a cycle
(corners first, then edges, centre and part tilts). The opponents play their own shots (their slices are the aim's
incoming cases). AIM_SINGLES=1: the player count (0x422fa4)
reads 2 over P1's contact, so the aim takes its singles width in a doubles save (no singles save exists).
Writes a pair of samples per aim: the frame before P1's +0x3e90 changes and the frame it does, each a `frames_live`
sample followed by P1's +0x12b0..+0x1320 (end, character, TParam aim values) and the last shot's record
(P1 +0x1400 → +0x1b0..+0x1c0). AIM_DEBUG=1 logs the approach. Needs tools/vpad.py serve (pcsx2-hst.sh starts it).
AIM_INCOMING=1 keeps only the aims struck off an incoming rally slice (for the `incoming` cases plain play rarely gives).
AIM_SERVE=1 records P1's serve aims instead (the swing's aim at the countdown's end, +0x3fa4 1, +0x3fa6 3): serving it
tosses with the next button (✕ mostly: the strong toss, whose mistiming nudges +0x3f10..+0x3f1c), holds the next stick
and swings AIM_SERVE_DELAY frames later (cycled, for every timing grade). Aims re-run by an instant replay (+0x4088
set: the replay's recorded stick, not the live pad) are never kept.
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
REACH = float(os.environ.get("AIM_REACH", "1.0"))
# AIM_LEAD: frames to the contact (contact_frame) to press at, cycled per press. Standing in reach the swing locks 2
# frames after the press with the countdown = those frames, so the offset is lead − 8: 8 sweet, 5 early, 11 late
AHEAD = float(os.environ.get("AIM_AHEAD", "0.8"))  # the contact point in front of P1 (m)
LEADS = [float(x) for x in os.environ.get("AIM_LEAD", "8,5,8,11").split(",")]

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
SHOTS = 4 + 0x90 + 0x423060 - 0x422f80  # the rally's shot count
SERVE = os.environ.get("AIM_SERVE") == "1"
SERVE_DELAYS = [int(x) for x in os.environ.get("AIM_SERVE_DELAY", "40,30,52,36,46,26,58").split(",")]
TOSSES = ("cross", "cross", "circle", "cross", "triangle")
def pf(smp, o, f="<i"): return struct.unpack_from(f, smp, AIM + o - 0x3e90)[0]  # P1 +o (0x3c00..0x4000) in a sample
def serve_aim(pre, cur):
    """The serve swing's aim: the countdown ends this frame with no miss (+0x3fa4 1, +0x3fa6 3, +0x3ec4 1 → −1)."""
    return pre[AIM + 0x3fa4 - 0x3e90] == 1 and pre[AIM + 0x3fa6 - 0x3e90] == 3 and pf(pre, 0x3ec4) == 1 and pre[AIM + 0x3ec8 - 0x3e90] == 0 and pf(cur, 0x3ec4) == -1
ONLY_INCOMING = os.environ.get("AIM_INCOMING") == "1"  # keep only aims struck off an incoming slice
def incoming(pre):
    """Some(was sweet) when the ball struck is a rally slice (the aim test's rule, from the last shot's record)."""
    rec = pre[-0x10:]
    hb, kind = struct.unpack_from("<2i", rec, 4)
    return (rec[12] != 0) if s32(struct.unpack_from("<I", pre, SHOTS)[0]) > 0 and hb < 4 and kind == 1 else None
def contact_frame(mz, s):
    """Frames until the game's predicted path (P1's shot record +0x1400: entries +0x50, count +0x54, now +0x58; 0x30
    each: pos, vel, bounces) comes nearest the contact depth AIM_AHEAD m in front of P1, as the search picks it."""
    rec = p.read32(me + 0x1400)
    buf, ln, now = p.read32(rec + 0x50), s32(p.read32(rec + 0x54)), s32(p.read32(rec + 0x58))
    m = min(ln - now, 40)
    if m <= 0: return None
    b, zc = p.read_block(buf + 0x30 * now, 0x30 * m), mz - s * AHEAD
    ks = [k for k in range(m) if s32(struct.unpack_from("<I", b, 0x30 * k + 0x20)[0]) <= 1 and f32(struct.unpack_from("<I", b, 0x30 * k + 8)[0]) * s > 0.5]
    return min(ks, key=lambda k: abs(f32(struct.unpack_from("<I", b, 0x30 * k + 8)[0]) - zc)) if ks else None

pressed_at = None
serve_at, serves = None, 0  # the toss's sample, the serves tossed
last, n, aims, prev_sample, pressed_for, held, idle, presses = p.read32(VSYNC), 0, 0, None, None, (0, 0), 0, 0
while aims < want and n < cap:
    v = p.next_frame(last)
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    a = p.settle(r, v)
    if a is None:  # the frame ticked while reading: skip it (the pair check needs consecutive frames)
        print(f"torn frame {v}", flush=True)
        continue
    sample = struct.pack("<I", v) + a
    n += 1
    while releases and releases[0][0] <= n: send(f"up {releases.pop(0)[1]}")
    replay = p.read8(me + 0x4088) != 0
    if SERVE and prev_sample and not replay and serve_aim(prev_sample, sample) and last - struct.unpack("<I", prev_sample[:4])[0] == 1:
        out.write(prev_sample + sample)
        out.flush()
        aims += 1
        t = struct.unpack("<4f", sample[AIM:AIM + 16])
        print(f"vsync {v}: serve aim {aims} toss {pf(sample, 0x3ea0)} stick {held} offset {pf(sample, 0x3fa0)} grade {sample[AIM + 0x3ee8 - 0x3e90]} branch {sample[BRANCH]} -> ({t[0]:.3f}, {t[2]:.3f}) nudge {struct.unpack_from('<4f', sample, AIM + 0x80)}", flush=True)
    if not SERVE and not replay and prev_sample and prev_sample[BRANCH] in (1, 2, 3, 4) and sample[AIM:AIM + 16] != prev_sample[AIM:AIM + 16] and last - struct.unpack("<I", prev_sample[:4])[0] == 1:
        inc = incoming(prev_sample)
        if not ONLY_INCOMING or inc is not None:
            out.write(prev_sample + sample)
            out.flush()
            aims += 1
        t = struct.unpack("<4f", sample[AIM:AIM + 16])
        print(f"vsync {v}: aim {aims} incoming {inc} branch {prev_sample[BRANCH]} kind {sample[BRANCH + 0x23]} stick {held} offset {s32(struct.unpack_from('<I', sample, AIM + 0x110)[0])} -> ({t[0]:.3f}, {t[2]:.3f})", flush=True)
    if os.environ.get("AIM_DEBUG") and prev_sample and (prev_sample[BRANCH] != sample[BRANCH] or sample[AIM:AIM + 16] != prev_sample[AIM:AIM + 16]):
        print(f"vsync {v}: branch {prev_sample[BRANCH]}->{sample[BRANCH]} aim {struct.unpack('<4f', sample[AIM:AIM + 16])}", flush=True)
    prev_sample = sample
    branch, count = p.read8(me + 0x3ec1), s32(p.read32(me + 0x3ec4))
    locked = branch in (1, 2, 3, 4) and count >= 0
    pos, vel = [f32(p.read32(ball + 0x120 + 4 * j)) for j in range(3)], [f32(p.read32(ball + 0x130 + 4 * j)) for j in range(3)]
    mx, mz = f32(p.read32(me + 0x3d70)), f32(p.read32(me + 0x3d78))
    hit = tuple(p.read32(ball + o) for o in (0x70, 0x74, 0x78))
    if os.environ.get("AIM_DEBUG") and n % 8 == 0 and vel[2] * mz > 0:
        print(f"  {v}: b{branch} c{count} ball ({pos[0]:.2f},{pos[1]:.2f},{pos[2]:.2f}) me ({mx:.2f},{mz:.2f}) held {held} eta {(mz - pos[2]) / vel[2]:.1f} p{presses}", flush=True)
    s = 1 if mz > 0 else -1  # the camera sits behind P1: on the +z half stick right is −x
    idle += 1
    if locked and pressed_at:
        print(f"vsync {v}: locked {v - pressed_at} frames after the press, countdown {count}", flush=True)
        pressed_at = None
    if locked:
        stick = STICKS[presses % len(STICKS)]
        if SINGLES and count <= 2 and p.read32(COUNT) != 2:
            p.write32(COUNT, 2)  # the aim's singles width, in this doubles save: just over P1's contact
    elif vel[2] * mz > 0:
        dx = pos[0] + vel[0] * (mz - pos[2]) / vel[2] - mx  # where it crosses P1's line, from P1
        dx -= REACH if dx > 0 else -REACH  # stand a reach off it, on the nearer side (lined up, nothing swings)
        stick = (round(-s * max(-1, min(1, 2 * dx)), 1), 0) if abs(dx) > 0.15 else (0, 0)
        k = contact_frame(mz, s)
        if k is not None and k <= LEADS[presses % len(LEADS)] and hit != pressed_for:
            pressed_for, idle = hit, 0
            presses += 1
            press(BUTTONS[presses % 3])
            pressed_at = v
            print(f"vsync {v}: press {BUTTONS[presses % 3]} side {dx + (REACH if dx > 0 else -REACH):.2f} m contact in {k} lead {LEADS[(presses - 1) % len(LEADS)]}", flush=True)
    else:
        stick = (0, 0)
    if SINGLES and not locked and p.read32(COUNT) != 4:
        p.write32(COUNT, 4)
    if stick != held:
        held = stick
        send(f"stick l {stick[0]} {stick[1]}")
    serving = p.read8(me + 0x3fa4) == 1 and not replay
    if SERVE and serving:
        sub = p.read8(me + 0x3fa6)
        if os.environ.get("AIM_DEBUG") and n % 4 == 0:
            print(f"  {v}: serving sub {sub} countdown {s32(p.read32(me + 0x3ec4))} toss {s32(p.read32(me + 0x3ea0))} at {serve_at} n {n}", flush=True)
        if serve_at is None and sub == 0 and idle > 60:
            serve_at, idle = n, 0
            stick = STICKS[serves % len(STICKS)]
            press(TOSSES[serves % len(TOSSES)])
            print(f"vsync {v}: toss {TOSSES[serves % len(TOSSES)]} stick {stick} delay {SERVE_DELAYS[serves % len(SERVE_DELAYS)]}", flush=True)
        elif serve_at is not None:
            stick = STICKS[serves % len(STICKS)]
            if n - serve_at == SERVE_DELAYS[serves % len(SERVE_DELAYS)]:
                press(TOSSES[serves % len(TOSSES)])
        else:
            stick = (0, 0)
        if stick != held:
            held = stick
            send(f"stick l {stick[0]} {stick[1]}")
        continue
    if serve_at is not None:  # the serve is away
        serve_at, serves = None, serves + 1
    if not locked and idle > (40 if branch == 5 else 120) and (branch == 5 or p.read8(gm + 0x55) != 3):  # never mid-rally
        idle = 0
        press("cross")
if SINGLES: p.write32(COUNT, 4)
send("release")
print("done", n, "frames", aims, "aims", presses, "presses")
