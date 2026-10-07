# Disassemble a VIF MPG upload (VU1 microcode) found in an ELF: vu1dis.py <elf> <file offset of the MPG code> [<vu addr base override>]
# Reuses vu0dis's upper/lower decoders; VU1-only lower ops (XGKICK, XTOP, XITOP) added here.
import os, sys, struct
sys.path.insert(0, os.path.dirname(__file__))
import vu0dis
def lower(w):
    if w >> 25 == 0x40 and w & 0x3f >= 0x3c:
        ext = ((w >> 6) & 31) << 2 | (w & 3)
        fs = (w >> 11) & 31; ft = (w >> 16) & 31
        if ext == 0x6c: return f'XGKICK vi{fs}'
        if ext == 0x68: return f'XTOP vi{ft}'
        if ext == 0x69: return f'XITOP vi{ft}'
    if w >> 25 == 1: return f'SQ.{vu0dis.dest(w)} vf{(w >> 11) & 31},{(w & 0x7ff) - (0x800 if w & 0x400 else 0)}(vi{(w >> 16) & 31})'
    if w >> 25 == 0x40 and w & 0x3f >= 0x3c and ((w >> 6) & 31) << 2 | (w & 3) in (0x35, 0x37): return f"{'SQI' if w & 3 == 1 else 'SQD'}.{vu0dis.dest(w)} vf{(w >> 11) & 31},(vi{(w >> 16) & 31}{'++' if w & 3 == 1 else '--'})"
    if w >> 25 in (0x24, 0x25): return f"{'JR' if w >> 25 == 0x24 else 'JALR'} vi{(w >> 11) & 31}" + (f',vi{(w >> 16) & 31}' if w >> 25 == 0x25 else '')
    return vu0dis.lower(w)
def dis(d, off, base=None):
    w, = struct.unpack_from('<I', d, off); num = (w >> 16) & 0xff or 256; addr = w & 0xffff if base is None else base
    out = []
    for i in range(num):
        lo, hi = struct.unpack_from('<II', d, off + 4 + i * 8)
        flags = ''.join(c for c, b in zip('IEMDT', (31, 30, 29, 28, 27)) if hi >> b & 1)
        if hi >> 31 & 1: l = f'LOI {struct.unpack("<f", struct.pack("<I", lo))[0]!r} ({lo:#x})'
        else: l = lower(lo)
        out.append(f'{addr + i:04x}  {vu0dis.upper(hi):40s} {l:36s} {flags}')
    return out
if __name__ == '__main__':
    d = open(sys.argv[1], 'rb').read()
    for l in dis(d, int(sys.argv[2], 16), int(sys.argv[3], 16) if len(sys.argv) > 3 else None): print(l)
