"""Per serve in anim_s05.bin: the stance motion (0x20) time when the walk/toss starts, and the motion sequence."""
import struct, sys
S = 4 + 0x100 + 4 * 0x484
d = open('context/fixtures/anim_s05.bin', 'rb').read()
n = len(d) // S
def blk(k, p): return d[k * S + 0x104 + p * 0x484:][:0x484]
for p in range(4):
    run = []
    for k in range(n):
        b = blk(k, p)
        st, mode = b[0x3a4], b[0x3a6]
        mo, t = struct.unpack_from('<i', b, 0x420)[0], struct.unpack_from('<f', b, 0x438)[0]
        if st == 1:
            run.append((k, mode, mo, t))
        elif run:
            seq = []
            for (kk, md, m, tt) in run:
                if not seq or seq[-1][1] != m or seq[-1][0] != md:
                    seq.append([md, m, kk, tt, tt])
                else:
                    seq[-1][4] = tt
            print(f"p{p} k{run[0][0]}..{run[-1][0]}: " + "  ".join(f"m{md}/{m:#x}@{kk} {a:.1f}->{b:.1f}" for md, m, kk, a, b in seq))
            run = []

print("--- stance runs: frames, wraps (t drops), t at the end, max t")
for p in range(4):
    prev = None; run = []
    for k in range(n):
        b = blk(k, p)
        mo, t, st = struct.unpack_from('<i', b, 0x420)[0], struct.unpack_from('<f', b, 0x438)[0], b[0x3a4]
        if st == 1 and mo == 0x20:
            run.append(t)
        elif run:
            wraps = sum(1 for a, c in zip(run, run[1:]) if c < a)
            print(f"p{p} end k{k} next {mo:#x}: {len(run)} frames, wraps {wraps}, t end {run[-1]:.2f}, max {max(run):.2f}, steps {sorted(set(round(c-a,3) for a,c in zip(run,run[1:]) if c>=a))[:4]}")
            run = []
