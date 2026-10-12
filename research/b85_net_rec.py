#!/usr/bin/env python3
"""B85: a net hit in the original and the word it pops up (THIP off the cord, THUP into the net, GONG off the post).
Usage: b85_net_rec.py <slot> <out.bin> <shot-dir> <vsync,vx,vy,vz> [frames=240].

Loads the state, overwrites the live ball's velocity once right after that vsync (as tools/record_live.py --vel; the
P15 velocities in context/live/p15/run.sh), then records every frame from 30 frames before it. Screenshots (tools/screenshot.sh, async: they
land ~10 frames late at 1x: run the copy slowed, HST_SPEED=0.1 tools/pcsx2-hst.sh) 1 and 8 frames after the first new word; their request
vsyncs go to <shot-dir>/shots.txt.

Sample: u32 vsync, live ball 0x290, its contact records *(ball+0x8f8) 3 x 0x50 (zeros while it has none), pop-up
object +0x130..+0x850 (0x720: entries 0x30 each from +0x130, count +0x6f0, last contact count +0x720, cooldown +0x840,
words made +0x844, soft-net flag +0x849), camera matrix 0x40, view matrix 0x40, fov (f32, padded to 8), game object 0x60 (phase +0x55)."""
import os, struct, subprocess, sys, time
sys.path.insert(0, "tools")
from pine import Pine

VSYNC, GM_PTR = 0x1d5780, 0x422f80
CAM, VIEW, FOV, VTABLE = 0x1e7f30, 0x1e7e30, 0x1e7d50, 0x1d1cc0
slot, out, shots = int(sys.argv[1]), open(sys.argv[2], "wb"), sys.argv[3]
poke = sys.argv[4].split(",")
want = int(sys.argv[5]) if len(sys.argv) > 5 else 240
os.makedirs(shots, exist_ok=True)
p = Pine()
p.load_state(slot)
time.sleep(2)  # the load lands asynchronously; pointers read before it are stale
gm = p.read32(GM_PTR)
ball = p.read32(gm + 0x88)
mgr = int(os.environ.get("HST_POPUPS", "0x1529db0"), 16)
assert p.read32(mgr) == VTABLE, "pop-up object not at %x" % mgr
head = [(ball, 0x290)]
tail = [(mgr + 0x130, 0x720), (CAM, 0x40), (VIEW, 0x40), (FOV, 8), (gm, 0x60)]
log = open(os.path.join(shots, "shots.txt"), "w")


def grab(v):
    c = p.read32(ball + 0x8f8)
    ok = 0x100000 <= c < 0x2000000 - 0xf0 and c % 8 == 0  # a stale pointer between points fails the batch
    a = p.settle(head + ([(c, 0xf0)] if ok else []) + tail, v)
    if a is None or ok:
        return a
    return a[:0x290] + bytes(0xf0) + a[0x290:]


last, n, words, spawned, pending = p.read32(VSYNC), 0, None, None, []
while n < want:
    v = p.next_frame(last)
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    if v < int(poke[0]) - 30:  # the load lands anywhere from ~7500 on
        continue
    a = grab(v)
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    out.write(struct.pack("<I", v) + a)
    n += 1
    made = struct.unpack_from("<i", a, 0x290 + 0xf0 + 0x844 - 0x130)[0]
    if v == int(poke[0]):
        for k, c in enumerate(poke[1:]):
            p.write32(ball + 0x130 + 4 * k, struct.unpack("<I", struct.pack("<f", float(c)))[0])
        print(f"velocity written after vsync {v}", flush=True)
        words = made
    if words is not None and spawned is None and made > words:
        spawned = n
        print(f"word made at vsync {v}", flush=True)
    if spawned is not None and n - spawned in (0, 7):
        f = os.path.join(shots, f"orig_{v}.png")
        pending.append((subprocess.Popen(["tools/screenshot.sh", f], stdout=subprocess.DEVNULL), f, v))
    for s in [s for s in pending if s[0].poll() is not None]:
        pending.remove(s)
        log.write(f"{s[1]} sent at vsync {s[2]}, copied by {v}\n")
print("done", n)
