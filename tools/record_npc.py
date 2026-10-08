#!/usr/bin/env python3
"""Record the walking spectators every frame from a save-state load (bot games: slot 5).
Usage: record_npc.py <slot> <frames> <out.bin> [trig|prox].
With `trig` the trigger creatures (vtable 0x1d2180, 0x290 bytes, controller at +0x5c) instead of the walkers. The walkers are found by vtable in the state's RAM (the .p2s file).
Header: u32 count, then count × u32 walker address. Sample = u32 vsync, 0x422f80 + 0x180 (globals), the shared MT
(*(*(*(gm+0x84)+0x154)+0x50), 0x9d0: index at +0x9c4), the gallery manager *(0x43b1c0) +0x680..+0x900 and
+0x1b60..+0x1b80, the judge *(0x42d6c0) +0x420 (8 bytes), then per walker its object (0x310), its animation
controller *(+0x60) (0x40) and that controller's animation header *(ctrl+0x24) (0x30). Stops when the match ends
(the match object or a walker goes away); a frame that ticks mid-read is skipped (missed).
`prox` is `trig` plus, at the end of each sample, what the startled creatures look at: the ball's position
*(gm+0x88)+0xe0, four players' positions *(gm+0xa8+4i)+0x3d70 (0x10 each), the per-type "startled this point"
flags *(0x43b820)+0x424 (0x40), then per creature the 8 bytes at *(+0xb8) (type 48's scrub state).
HST_PAD="press cross 100;sleep 500;..." sends those tools/vpad.py commands right after the load (a human P1).
HST_POKE="<sample>:<k>:<b|0..3>:<dx>:<dz>;..." moves creature k (header order) to the ball's or player i's position
plus (dx, 0, dz) after that sample, so a frozen creature finds someone close."""
import os, subprocess, struct, sys, time, zipfile
from pine import Pine

VSYNC, GM_PTR, WALKER = 0x1d5780, 0x422f80, 0x1d1de0
STATES = "/home/ryha/Emulation/saves/ps2/states/SCUS-97610 (72326E67).%02d.p2s"
if os.environ.get("HST_PCSX2"):  # parallel runs: copy N's own states (tools/pcsx2-hst.sh)
    common = subprocess.check_output(["git", "-C", os.path.dirname(__file__), "rev-parse", "--path-format=absolute",
                                      "--git-common-dir"], text=True).strip()
    STATES = f"{os.path.dirname(common)}-slots/pcsx2/s{os.environ['HST_PCSX2']}/PCSX2/sstates/" + os.path.basename(STATES)
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
prox = sys.argv[4:] == ["prox"]
if sys.argv[4:] in (["trig"], ["prox"]):
    WALKER, SIZE, CTRL = 0x1d2180, 0x290, 0x5c
else:
    SIZE, CTRL = 0x310, 0x60
ram = zipfile.ZipFile(STATES % slot).read("eeMemory.bin")
walkers = [a for a in range(0x100000, len(ram), 4) if struct.unpack_from("<I", ram, a)[0] == WALKER]
out.write(struct.pack(f"<I{len(walkers)}I", len(walkers), *walkers))

p = Pine(step=True)
p.load_state(slot)
time.sleep(0.3)
if pad := os.environ.get("HST_PAD"):  # drive a human P1 from the load: tools/vpad.py commands, ;-separated
    subprocess.Popen([os.path.join(os.path.dirname(__file__), "vpad.py"), "send", *pad.split(";")])

def regions():
    gm, mgr = p.read32(GM_PTR), p.read32(0x43b1c0)
    r = [(GM_PTR, 0x180), (p.read32(p.read32(p.read32(gm + 0x84) + 0x154) + 0x50), 0x9d0),
         (mgr + 0x680, 0x280), (mgr + 0x1b60, 0x20), (p.read32(0x42d6c0) + 0x420, 8)]
    for w in walkers:
        c = p.read32(w + CTRL)  # 0 for a trigger creature without a model: reads scratch RAM
        r += [(w, SIZE), (c, 0x40), (p.read32(c + 0x24), 0x30)]
    if prox:
        r.append((p.read32(gm + 0x88) + 0xe0, 0x10))
        r += [(p.read32(gm + 0xa8 + 4 * i) + 0x3d70, 0x10) for i in range(4)]
        r.append((p.read32(0x43b820) + 0x424, 0x40))
        r += [(p.read32(w + 0xb8) if 0x100000 <= p.read32(w + 0xb8) < 0x2000000 else 0x100000, 8) for w in walkers]
    return r

# HST_POKE="<sample>:<creature k>:<b|player i>:<dx>:<dz>;...": move a creature next to the ball or a player
pokes = [(int(a), int(k), w, float(x), float(z)) for a, k, w, x, z in
         (q.split(":") for q in os.environ.get("HST_POKE", "").split(";") if q)]
last, n, missed, gm0 = p.read32(VSYNC), 0, 0, p.read32(GM_PTR)
while n < want:
    v = p.next_frame(last)
    last = v
    if p.read32(GM_PTR) != gm0:
        sys.exit(f"match object gone at vsync {v} (match over), {n} samples")
    if any(p.read32(w) != WALKER for w in walkers):
        sys.exit(f"walkers gone at vsync {v} (match over), {n} samples")
    a = p.settle(regions(), v)
    if a is None:
        missed += 1
        if missed > 300: sys.exit(f"300 frames in a row missed at vsync {v}, {n} samples")
        continue
    missed = 0
    out.write(struct.pack("<I", v) + a)
    out.flush()
    n += 1
    for at, k, who, dx, dz in pokes:
        if at == n:  # the creature's world position to `who`'s plus (dx, 0, dz); it stays put while it stands
            gm = p.read32(GM_PTR)
            src = p.read32(gm + 0x88) + 0xe0 if who == "b" else p.read32(gm + 0xa8 + 4 * int(who)) + 0x3d70
            x, y, z = struct.unpack("<3f", struct.pack("<3I", *(p.read32(src + 4 * c) for c in range(3))))
            for c, val in enumerate((x + dx, y, z + dz)):
                p.write32(walkers[k] + 0x1e0 + 4 * c, struct.unpack("<I", struct.pack("<f", val))[0])
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
