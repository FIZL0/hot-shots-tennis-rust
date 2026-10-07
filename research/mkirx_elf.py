#!/usr/bin/env python3
"""IOP module (.IRX) -> plain MIPS ELF Ghidra can analyse.

IRX sections are linked at 0 and relocated at load; at base 0 the relocations add nothing, so the image is the
sections at their addresses. Function starts (jal targets, `addiu sp,sp,-N` after a jr ra) go in as symbols so
Ghidra's analysis has seeds. Usage: mkirx_elf.py IN.IRX OUT.elf
"""
import struct, sys

src = open(sys.argv[1], 'rb').read()
shoff, = struct.unpack_from('<I', src, 0x20)
shnum, = struct.unpack_from('<H', src, 0x30)
img = bytearray()
text_size = 0
entry = 0
for i in range(shnum):
    name, typ, flags, addr, off, size = struct.unpack_from('<6I', src, shoff + i * 40)
    if typ == 0x70000080:  # .iopmod: module info, entry, gp, text size, ...
        entry, = struct.unpack_from('<I', src, off + 4)
    if flags & 2:  # SHF_ALLOC
        if len(img) < addr + size:
            img.extend(bytes(addr + size - len(img)))
        if typ != 8:  # not NOBITS
            img[addr:addr + size] = src[off:off + size]
        if flags & 4:
            text_size = addr + size

funcs = {entry}
words = struct.unpack_from('<%dI' % (text_size // 4), img)
for i, w in enumerate(words):
    if w >> 26 == 3:  # jal
        funcs.add((w & 0x3ffffff) << 2)
    if (w & 0xffff8000) == 0x27bd8000 and i and words[i - 1] in (0x03e00008,) or \
       (w & 0xffff8000) == 0x27bd8000 and i > 1 and words[i - 2] == 0x03e00008:
        funcs.add(i * 4)
funcs = sorted(f for f in funcs if f < text_size)

strtab = b'\0' + b''.join(b'f_%05x\0' % f for f in funcs)
syms = bytes(16)
pos = 1
for f in funcs:
    syms += struct.pack('<IIIBBH', pos, f, 0, 0x12, 0, 1)  # GLOBAL FUNC, section 1
    pos += len(b'f_%05x\0' % f)
shstr = b'\0.text\0.symtab\0.strtab\0.shstrtab\0'
data_off = 0x1000
sym_off = data_off + len(img)
str_off = sym_off + len(syms)
shs_off = str_off + len(strtab)
sh_off = (shs_off + len(shstr) + 3) & ~3
ehdr = struct.pack('<16sHHIIIIIHHHHHH', b'\x7fELF\x01\x01\x01' + bytes(9), 2, 8, 1, entry, 52, sh_off, 0x1000,
                   52, 32, 1, 40, 5, 4)
phdr = struct.pack('<8I', 1, data_off, 0, 0, len(img), len(img), 7, 0x1000)
sh = bytes(40)
sh += struct.pack('<10I', 1, 1, 7, 0, data_off, len(img), 0, 0, 16, 0)
sh += struct.pack('<10I', 7, 2, 0, 0, sym_off, len(syms), 3, 1, 4, 16)
sh += struct.pack('<10I', 15, 3, 0, 0, str_off, len(strtab), 0, 0, 1, 0)
sh += struct.pack('<10I', 23, 3, 0, 0, shs_off, len(shstr), 0, 0, 1, 0)
out = bytearray(ehdr + phdr)
out += bytes(data_off - len(out)) + img + syms + strtab + shstr
out += bytes(sh_off - len(out)) + sh
open(sys.argv[2], 'wb').write(out)
print(f'{len(funcs)} functions, entry {entry:#x}, image {len(img):#x}')
