#!/usr/bin/env python3
"""B42c: start the court 8 doubles match from save slot 2 (as b42a_plate_rec.py) and record, every vsync from the
last menu screen to the end of the court intro (match phase 0), the sound side: the 8 SMF player slots and the
sound driver's command ring, beside the match frame, phase and the intro overlay's state (mgr+0xd94).
Sample = u32 vsync, i32 match frame (-1 = no match yet), u8 phase, u8 d94, 2 pad, 8 SMF slots (8 x 0x80), u32 n,
n ring commands (16 bytes). Optionally press ✕ once the match frame passes <skip frame>. B42C_MUSIC=1 keeps the
match music option on until the intro's first frame (the save has it off; bytes 0x423206 and its match copy 0x4230c6).
Usage (under tools/pcsx2.sh): b42c_sound_rec.py <out.bin> [skip frame]"""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad
VSYNC, RING, HEAD, SMF = 0x1d5780, 0x305000, 0x304fc0, 0x310e40
out = open(sys.argv[1], "wb"); skip = int(sys.argv[2]) if len(sys.argv) > 2 else -1
p = Pine()
p.load_state(2)
time.sleep(1.5)
music = os.environ.get("B42C_MUSIC") == "1"
on = lambda a: p.write32(a & ~3, p.read32(a & ~3) & ~(0xff << 8 * (a & 3)) | 1 << 8 * (a & 3))
vpad("sleep 2500", "press cross 150", "sleep 1200", *["press cross 150", "sleep 1200"] * 3, "press start 150", "sleep 1500")
last, head, missed, n, pressed, t = p.read32(VSYNC), p.read32(HEAD) % 1024, 0, 0, False, time.time()
done_at = None
while time.time() - t < 120:
    v = p.read32(VSYNC)
    if v == last:
        continue
    missed += max(0, v - last - 1); last = v
    gm = p.read32(0x422f80)
    if music and (not gm or p.read32(gm + 0x58) == 0):
        on(0x423206); on(0x4230c6)
    fr, ph, d94 = -1, 0xff, 0
    if gm:
        ph, fr = p.read8(gm + 0x55), p.read32(gm + 0x58)
        mgr = p.read32(0x43b1d8)
        d94 = mgr and p.read8(mgr + 0xd94)
    a = p.read_regions([(SMF, 0x400), (HEAD, 8)])
    h = struct.unpack_from("<I", a, 0x400)[0] % 1024
    k = (h - head) % 1024
    rg = [(RING + 16 * head, 16 * min(k, 1024 - head))] + ([(RING, 16 * (head + k - 1024))] if head + k > 1024 else [])
    cmds = p.read_regions(rg) if k else b""
    head = h
    out.write(struct.pack("<IiBBxx", v, fr, ph, d94) + a[:0x400] + struct.pack("<I", k) + cmds)
    n += 1
    if not gm and time.time() - t < 25 and n % 150 == 0:
        vpad("press cross 150")
    if skip >= 0 and gm and fr >= skip and not pressed:
        vpad("press cross 100"); pressed = True
    if gm and ph == 2 and done_at is None:
        done_at = n
    if done_at is not None and n - done_at > 240:
        break
print("done", n, "missed", missed, "court", p.read32(0x422f90), "players", p.read32(0x422fa4))
