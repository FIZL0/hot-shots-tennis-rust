#!/bin/sh
# usage: dis.sh <hexstart> <hexend>  -> MIPS disassembly of the GAME overlay (context/elf/hst_game.elf), one line per
# word; llvm-mc for MIPS III, the R5900 float ops (adda/mula/madd/msub…, max/min, rsqrt) decoded here.
exec python3 - "$1" "$2" "$(dirname "$(realpath "$0")")/../context/elf/hst_game.elf" <<'PY'
import struct, subprocess, sys
s, e, f = int(sys.argv[1], 16), int(sys.argv[2], 16), sys.argv[3]
d = open(f, 'rb').read()
for va, off, n in [(0x100000, 0x100, 0xd2e80), (0x322d00, 0xd31d0, 0x100280)]:
    if va <= s < va + n:
        words = struct.unpack('<%dI' % ((e - s) // 4), d[off + s - va: off + e - va])
R59 = {0x18: 'adda.s', 0x19: 'suba.s', 0x1a: 'mula.s', 0x1c: 'madd.s', 0x1d: 'msub.s', 0x1e: 'madda.s',
       0x1f: 'msuba.s', 0x28: 'max.s', 0x29: 'min.s', 0x16: 'rsqrt.s'}
def r5900(w):
    if w >> 26 != 0x11 or (w >> 21) & 31 != 0x10: return None
    fn, ft, fs, fd = w & 63, (w >> 16) & 31, (w >> 11) & 31, (w >> 6) & 31
    if fn not in R59: return None
    op = R59[fn]
    if op in ('adda.s', 'suba.s', 'mula.s', 'madda.s', 'msuba.s'): return f'{op}\t$f{fs}, $f{ft}'
    return f'{op}\t$f{fd}, $f{fs}, $f{ft}'
out = []
for k, w in enumerate(words):
    t = r5900(w)
    if t is None:
        r = subprocess.run(['llvm-mc', '--disassemble', '--triple=mipsel', '--mcpu=mips3'],
                           input=' '.join('0x%02x' % b for b in struct.pack('<I', w)), capture_output=True, text=True)
        lines = [l.strip() for l in r.stdout.splitlines() if l.strip() and not l.strip().startswith('.text')]
        t = lines[0] if lines else '.word 0x%08x' % w
    out.append('%06x  %08x  %s' % (s + 4 * k, w, t))
print('\n'.join(out))
PY
