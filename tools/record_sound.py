#!/usr/bin/env python3
"""Record the sound command ring every frame from a save-state load, next to the live ball (bot games: slot 5).
Usage: record_sound.py <slot> <frames> <out.bin> [hits]. Run PCSX2 slowed down ([Framerate] NominalScalar = 0.25).
Sample = record_live.py's (u32 vsync + live ball 0x290 + predictor 0x290 + rally block 0x40), the live ball's
first 6 contact records (6 × 0x50, from the pointer at ball+0x8f8; zeros without one), then the sound library's
last positional play (angle, distance, volume: 3 × i32), u32 n and the n ring commands
({cmd, voice, word8, wordC}) the EE wrote since the previous sample. With `hits`, the hit sound's inputs go
between the play and n: effects object +0xb8..+0x100 (0xd0 shot kind, per-player 8-byte records at +0xd8),
0x422fa0..0x422fc0 (player count, character ids), 0x423040..0x423080 (hitter, hit count), gm+0x340 8 bytes,
the court marker matrix 8 (*(gm+0xb8) + 0xd130 + 8·0x40, 0x40) and players 0/1 +0x3e90..+0x3fb0."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, SND_PTR, RING, HEAD = 0x1d5780, 0x422f80, 0x312540, 0x305000, 0x304fc0
p, want, out = Pine(), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(int(sys.argv[1]))
time.sleep(0.3)
gm, snd = p.read32(GM_PTR), p.read32(SND_PTR)
ball = p.read32(gm + 0x88)
hits = "hits" in sys.argv[4:]
r = [(p.read32(gm + 0x88), 0x290), (p.read32(gm + 0x98), 0x290), (0x3165f0, 0x40), (snd + 8, 16)]
if hits:
    r += [(p.read32(0x423f80) + 0xb8, 0x48), (0x422fa0, 0x20), (0x423040, 0x40), (gm + 0x340, 8),
          (p.read32(gm + 0xb8) + 0xd130 + 8 * 0x40, 0x40)] + [(p.read32(gm + 0xa8 + 4 * i) + 0x3e90, 0x120) for i in range(2)]
r += [(HEAD, 8)]
last, n, head = p.read32(VSYNC), 0, p.read32(HEAD) % 1024
while n < want:
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
    h = struct.unpack_from("<I", a, len(a) - 8)[0] % 1024
    k = (h - head) % 1024
    cmds = p.read_regions([(RING + 16 * head, 16 * min(k, 1024 - head)), (RING, 16 * max(0, head + k - 1024))]) if k else b""
    head = h
    c = p.read32(ball + 0x8f8)
    contacts = p.read_regions([(c, 6 * 0x50)]) if c else bytes(6 * 0x50)
    fixed = 4 + 0x290 * 2 + 0x40
    out.write(struct.pack("<I", v) + a[:fixed - 4] + contacts + a[fixed:-8] + struct.pack("<I", k) + cmds)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
