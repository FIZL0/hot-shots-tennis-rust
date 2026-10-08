#!/usr/bin/env python3
"""Force a ball-hits-player pop-up in a rally and record it every frame, with screenshots along the way.
Usage: record_bodyhit.py <slot> <out.bin> <shot-dir> [player].

Loads the state, waits for the rally phase and 30 more frames, then sets the game's "player the ball hit" to
`player` (default 0) the way its body-hit handler does; the pop-up object spawns its word at the ball. Records until
the pop-up has gone and 30 frames more (at most 900 frames). Screenshots (tools/screenshot.sh, async) at the 1st, 6th,
12th, 40th and 120th recorded frame after the hit; their vsyncs go to <shot-dir>/shots.txt.

Sample: u32 vsync, game object 0x60 (tick +0x58, phase +0x55), pop-up object +0x140 (0x70: the first two entries from +0x150) and +0x6f0 (0x60: count, scale +0x740, rise +0x744, frames +0x748), camera matrix (0x40 at
its rows) and the view matrix (0x40), fov (f32, padded to 8)."""
import os, struct, subprocess, sys, time
from pine import Pine

VSYNC, GM_PTR, HIT = 0x1d5780, 0x422f80, 0x42305c
CAM, VIEW, FOV, VTABLE = 0x1e7f30, 0x1e7e30, 0x1e7d50, 0x1d1cc0
slot, out, shots = int(sys.argv[1]), open(sys.argv[2], "wb"), sys.argv[3]
who = int(sys.argv[4]) if len(sys.argv) > 4 else 0
os.makedirs(shots, exist_ok=True)
p = Pine()
p.load_state(slot)
time.sleep(2)
gm = p.read32(GM_PTR)
# the pop-up object: found by its vtable pointer in the save state (stable across loads of one state)
mgr = int(os.environ.get("HST_POPUPS", "0x1529db0"), 16)
assert p.read32(mgr) == VTABLE, "pop-up object not at %x" % mgr
while p.read8(gm + 0x55) != 3:
    time.sleep(0.05)
v0 = p.read32(VSYNC)
while p.read32(VSYNC) < v0 + 30:
    time.sleep(0.005)
p.write32(HIT, who)
print("hit set at vsync", p.read32(VSYNC), flush=True)
r = [(gm, 0x60), (mgr + 0x140, 0x70), (mgr + 0x6f0, 0x60), (CAM, 0x40), (VIEW, 0x40), (FOV, 8)]
last, n, gone, log = p.read32(VSYNC), 0, None, open(os.path.join(shots, "shots.txt"), "w")
while n < 900 and (gone is None or n < gone + 30):
    v = p.read32(VSYNC)
    if v == last:
        continue
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    time.sleep(0.004)
    a = p.settle(r, v)
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    out.write(struct.pack("<I", v) + a)
    n += 1
    count = struct.unpack_from("<i", a, 0x60 + 0x70)[0]
    if gone is None and n > 5 and count == 0:
        gone = n
        print("pop-up gone at frame", n, flush=True)
    if n in (1, 6, 12, 40, 120):
        f = os.path.join(shots, f"orig_{n:03d}.png")
        subprocess.Popen(["tools/screenshot.sh", f], stdout=subprocess.DEVNULL)
        log.write(f"{f} sent at vsync {v}\n")
print("done", n)
