#!/usr/bin/env python3
"""Rebuild the sun-shade map from an exact frame after a save state, keeping the reds (P17v4).
Usage: tools/pcsx2.sh env HST_LOCKSTEP=1 python3 research/p17v4_frame.py <slot> <frames> <out-prefix>

Loads the state, FrameAdvances <frames> vsyncs, and while paused snapshots EE RAM (<out>.ee, 32 MB) and patches
the read-back loop as p17v_rebuild.py does, sets the build countdown (+0x3c) to 1, then lets the game run until
the build is over. Writes <out>.red (1280 × 896 reds, row = game x). Patches are removed afterwards."""
import os, sys, time
sys.path.insert(0, "tools")
from pine import Pine

VSYNC, RAW, LOOP = 0x1d5780, 0x1e10000, 0x33cbfc
T1, T3, T4, A1 = 9, 11, 12, 5
def sll(rd, rt, sa): return rt << 16 | rd << 11 | sa << 6
def addu(rd, rs, rt): return rs << 21 | rt << 16 | rd << 11 | 0x21
def lui(rt, imm): return 0x0f << 26 | rt << 16 | imm & 0xffff
def ori(rt, rs, imm): return 0x0d << 26 | rs << 21 | rt << 16 | imm & 0xffff
def sb(rt, off, rs): return 0x28 << 26 | rs << 21 | rt << 16 | off & 0xffff

slot, frames, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
p = Pine(step=True)
p.load_state(slot)
v = p.next_frame(None)
for _ in range(frames):
    v = p.next_frame(v)
sh = p.read32(p.read32(p.read32(0x422f80) + 0x84) + 0x138)
mp = p.read32(sh + 0x28)
if len(sys.argv) < 5:  # ponytail: whole EE RAM, 32 MB over PINE (~1 min); pass any 4th arg to skip it
    open(out + ".ee", "wb").write(b"".join(p.read_block(o, 0x10000) for o in range(0, 0x2000000, 0x10000)))
p.pause()
orig = [p.read32(LOOP + 4 * k) for k in range(7)]
k = (RAW - mp * 8) & 0xffffffff
patch = [sll(T3, A1, 3), addu(T3, T3, T1), lui(T4, k >> 16), ori(T4, T4, k), addu(T3, T3, T4), sb(7, 0, T3), 0]
for i, w in enumerate(patch):
    p.write32(LOOP + 4 * i, w)
print("countdown", p.read32(sh + 0x3c), "proj", [hex(p.read32(sh + o)) for o in range(0x2c, 0x3c, 4)])
if os.environ.get("DUMP"):  # GS-dump (F11 = GSDumpSingleFrame in this copy) from just before the build
    n = os.environ["HST_PCSX2"]
    pid = open(os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), f"hst-pcsx2{n}.pid")).read().strip()
    import subprocess
    subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F11", window = "pid:{pid}" }})'], check=True)
if os.environ.get("RUNWRITE"):  # as p17v_capture.py: countdown written while the game runs
    p.resume(); p.write32(sh + 0x3c, 1)
else:
    p.write32(sh + 0x3c, 1); p.resume()
try:
    t = time.monotonic()
    while p.read32(sh + 0x3c) != 0:
        if time.monotonic() - t > 20: sys.exit("countdown never ran out")
        time.sleep(0.01)
    v = p._u32(VSYNC)
    while p._u32(VSYNC) - v < 150:
        time.sleep(0.05)
finally:
    p.pause()
    for i, w in enumerate(orig):
        p.write32(LOOP + 4 * i, w)
print("proj after", [hex(p.read32(sh + o)) for o in range(0x2c, 0x3c, 4)])
red = b"".join(p.read_block(RAW + o, min(0x10000, 0x500 * 0x380 - o)) for o in range(0, 0x500 * 0x380, 0x10000))
open(out + ".red", "wb").write(red)
print(out, "vsync", v, "red>0x6f", sum(r > 0x6f for r in red))
