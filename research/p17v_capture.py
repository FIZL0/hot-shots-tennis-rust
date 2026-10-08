#!/usr/bin/env python3
"""GS-dump a sun-shade map build and keep its read-back reds, from the same build (P17v).
Usage: tools/pcsx2.sh python3 research/p17v_capture.py <slot> <out-prefix> [late-vsyncs]

As p17v_rebuild.py (read-back loop patched to store each pixel's red at RAW), but the build also runs under
copy N's GSDumpSingleFrame (F11). With late-vsyncs the dump is started that many vsyncs after the build began, to
catch the later tiles. Writes <out>.red and moves the dump to <out>.gs.zst. Patches are removed afterwards."""
import os, shutil, subprocess, sys, time
sys.path.insert(0, "tools")
sys.argv, args = sys.argv[:1], sys.argv[1:]
from pine import Pine

VSYNC, RAW, LOOP = 0x1d5780, 0x1e10000, 0x33cbfc
T1, T3, T4, A1 = 9, 11, 12, 5
def sll(rd, rt, sa): return rt << 16 | rd << 11 | sa << 6
def addu(rd, rs, rt): return rs << 21 | rt << 16 | rd << 11 | 0x21
def lui(rt, imm): return 0x0f << 26 | rt << 16 | imm & 0xffff
def ori(rt, rs, imm): return 0x0d << 26 | rs << 21 | rt << 16 | imm & 0xffff
def sb(rt, off, rs): return 0x28 << 26 | rs << 21 | rt << 16 | off & 0xffff

slot, out, late = int(args[0]), args[1], int(args[2]) if len(args) > 2 else 0
n = os.environ["HST_PCSX2"]
snaps = f"../pcsx2/s{n}/PCSX2/snaps"
pid = open(os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), f"hst-pcsx2{n}.pid")).read().strip()
f11 = ["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F11", window = "pid:{pid}" }})']
p = Pine()
p.load_state(slot)
v = p.read32(VSYNC)
while p.read32(VSYNC) - v < 8:
    time.sleep(0.05)
sh = p.read32(p.read32(p.read32(0x422f80) + 0x84) + 0x138)
mp = p.read32(sh + 0x28)
orig = [p.read32(LOOP + 4 * k) for k in range(7)]
k = (RAW - mp * 8) & 0xffffffff
patch = [sll(T3, A1, 3), addu(T3, T3, T1), lui(T4, k >> 16), ori(T4, T4, k), addu(T3, T3, T4), sb(7, 0, T3), 0]
p.pause()
for i, w in enumerate(patch):
    p.write32(LOOP + 4 * i, w)
p.resume()
before = set(os.listdir(snaps))
try:
    p.write32(sh + 0x3c, 1)
    if late:
        while p.read32(sh + 0x3c) != 0:
            time.sleep(0.005)
        v = p.read32(VSYNC)
        while p.read32(VSYNC) - v < late:
            time.sleep(0.005)
    subprocess.run(f11, check=True)
    v = p.read32(VSYNC)
    while p.read32(VSYNC) - v < 120:
        time.sleep(0.05)
finally:
    p.pause()
    for i, w in enumerate(orig):
        p.write32(LOOP + 4 * i, w)
    p.resume()
red = b"".join(p.read_block(RAW + o, min(0x10000, 0x500 * 0x380 - o)) for o in range(0, 0x500 * 0x380, 0x10000))
open(out + ".red", "wb").write(red)
for _ in range(120):
    new = [f for f in set(os.listdir(snaps)) - before if f.endswith(".gs.zst")]
    if new: break
    time.sleep(0.5)
time.sleep(3)
shutil.move(os.path.join(snaps, new[0]), out + ".gs.zst")
print("dump", new[0], "red>0x6f", sum(r > 0x6f for r in red))
