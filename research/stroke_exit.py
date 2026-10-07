"""N1f: how strokes end in anim_s05 — per stroke motion (0x10..0x1d) left for another, the sampled time vs the clip length."""
import struct, collections
d = open('context/fixtures/anim_s05.bin', 'rb').read()
S = 4 + 0x100 + 4 * 0x484
rd = lambda k, p: d[k * S + 0x104 + p * 0x484:][:0x484]
I = lambda b, o: struct.unpack_from('<i', b, o)[0]
F = lambda b, o: struct.unpack_from('<f', b, o)[0]
stats = collections.Counter()
for p in range(4):
    for k in range(1, len(d) // S):
        a, b = rd(k - 1, p), rd(k, p)
        ma, mb = I(a, 0x420), I(b, 0x420)
        if ma != mb and 0x10 <= ma <= 0x1d:
            left = F(a, 0x480) - F(a, 0x438)
            key = (hex(ma), hex(mb), 'end' if left <= 1 else 'cut')
            stats[key] += 1
            if stats[key] <= 2:
                print(k, p, key, 'sampled', F(a, 0x438), 'len', F(a, 0x480), 'speed', F(a, 0x440))
for k, v in sorted(stats.items()): print(k, v)

# player +0x3c00.. is the block's first 0x400 bytes: +0x3f00 frames since contact, +0x3e50 recovery, +0x3ec4 countdown
P = lambda b, o: I(b, o - 0x3c00)
seen = collections.Counter()
for p in range(4):
    for k in range(1, len(d) // S):
        a, b = rd(k - 1, p), rd(k, p)
        ma, mb = I(a, 0x420), I(b, 0x420)
        if ma != mb and 0x10 <= ma <= 0x1d:
            cut = F(a, 0x480) - F(a, 0x438) > 1
            row = ('cut' if cut else 'end', P(a, 0x3f00), P(b, 0x3f00), P(b, 0x3e50), P(a, 0x3ec4), P(b, 0x3ec4))
            seen[row] += 1
for r, n in sorted(seen.items()): print(r, n)

# played-out strokes: frames since contact at the switch vs the clip length (contact at frame 8; the soft
# follow-through starts after contact, held 8 frames)
rows = collections.Counter()
for p in range(4):
    for k in range(1, len(d) // S):
        a, b = rd(k - 1, p), rd(k, p)
        ma, mb = I(a, 0x420), I(b, 0x420)
        if ma != mb and 0x10 <= ma <= 0x1d and F(a, 0x480) - F(a, 0x438) <= 1:
            rows[(hex(ma), P(a, 0x3f00) - int(F(a, 0x480)), P(a, 0x3e50) if False else None)] += 1
for r, n in sorted(rows.items()): print('end', r, n)
