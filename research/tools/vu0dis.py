# Minimal VU micro-mode disassembler (upper/lower) for reading short VU0 programs.
import struct, sys
BC='xyzw'
def dest(w): return ''.join(c for i,c in enumerate('xyzw') if w>>(24-i)&1)
UP={0x00:'ADD',0x04:'SUB',0x08:'MADD',0x0c:'MSUB',0x10:'MAX',0x14:'MINI',0x18:'MUL'}
UPN={0x1c:'MULq',0x1d:'MAXi',0x1e:'MULi',0x1f:'MINIi',0x20:'ADDq',0x21:'MADDq',0x22:'ADDi',0x23:'MADDi',0x24:'SUBq',0x25:'MSUBq',0x26:'SUBi',0x27:'MSUBi',0x28:'ADD',0x29:'MADD',0x2a:'MUL',0x2b:'MAX',0x2c:'SUB',0x2d:'MSUB',0x2e:'OPMSUB',0x2f:'MINI'}
UPX={0x00:'ADDA',0x04:'SUBA',0x08:'MADDA',0x0c:'MSUBA',0x18:'MULA'}
UPXN={0x10:'ITOF0',0x11:'ITOF4',0x12:'ITOF12',0x13:'ITOF15',0x14:'FTOI0',0x15:'FTOI4',0x16:'FTOI12',0x17:'FTOI15',0x1c:'MULAq',0x1d:'ABS',0x1e:'MULAi',0x1f:'CLIP',0x20:'ADDAq',0x21:'MADDAq',0x22:'ADDAi',0x23:'MADDAi',0x24:'SUBAq',0x25:'MSUBAq',0x26:'SUBAi',0x27:'MSUBAi',0x28:'ADDA',0x29:'MADDA',0x2a:'MULA',0x2c:'SUBA',0x2d:'MSUBA',0x2e:'OPMULA',0x2f:'NOP'}
def upper(w):
    op=w&0x3f; ft=(w>>16)&31; fs=(w>>11)&31; fd=(w>>6)&31; d=dest(w)
    if op>=0x3c:
        ext=((w>>6)&31)<<2 | (op&3)
        if ext in UPXN: n=UPXN[ext]; return f'{n}.{d} ' + ('' if n=='NOP' else f'vf{ft},vf{fs}' if 'ITOF' in n or 'FTOI' in n or n=='ABS' else f'ACC,vf{fs},vf{ft}')
        b=ext&~3
        if b in UPX: return f'{UPX[b]}{BC[ext&3]}.{d} ACC,vf{fs},vf{ft}{BC[ext&3]}'
        return f'?up-ext {ext:#x}'
    if op in UPN: return f'{UPN[op]}.{d} vf{fd},vf{fs},vf{ft}'
    b=op&~3
    if b in UP: return f'{UP[b]}{BC[op&3]}.{d} vf{fd},vf{fs},vf{ft}{BC[op&3]}'
    return f'?up {op:#x}'
LOWX={0x30:'MOVE',0x31:'MR32',0x34:'LQI',0x35:'SQI',0x36:'LQD',0x37:'SQD',0x38:'DIV',0x39:'SQRT',0x3a:'RSQRT',0x3b:'WAITQ',0x3c:'MTIR',0x3d:'MFIR',0x3e:'ILWR',0x3f:'ISWR',0x40:'RNEXT',0x41:'RGET',0x42:'RINIT',0x43:'RXOR'}
LOW={0x00:'LQ',0x01:'SQ',0x04:'ILW',0x05:'ISW',0x08:'IADDIU',0x09:'ISUBIU',0x10:'FCEQ',0x11:'FCSET',0x12:'FCAND',0x13:'FCOR',0x14:'FSEQ',0x15:'FSSET',0x16:'FSAND',0x17:'FSOR',0x18:'FMEQ',0x1a:'FMAND',0x1b:'FMOR',0x1c:'FCGET',0x20:'B',0x21:'BAL',0x24:'JR',0x25:'JALR',0x28:'IBEQ',0x29:'IBNE',0x2c:'IBLTZ',0x2d:'IBGTZ',0x2e:'IBLEZ',0x2f:'IBGEZ'}
def lower(w):
    top=w>>25; ft=(w>>16)&31; fs=(w>>11)&31; fd=(w>>6)&31; d=dest(w)
    if top==0x40:
        op=w&0x3f
        if op==0x30: return f'IADD vi{fd},vi{fs},vi{ft}'
        if op==0x31: return f'ISUB vi{fd},vi{fs},vi{ft}'
        if op==0x32: return f'IADDI vi{ft},vi{fs},{((w>>6)&31)-32 if (w>>10)&1 else (w>>6)&31}'
        if op==0x34: return f'IAND vi{fd},vi{fs},vi{ft}'
        if op==0x35: return f'IOR vi{fd},vi{fs},vi{ft}'
        if op>=0x3c:
            ext=((w>>6)&31)<<2 | (op&3); n=LOWX.get(ext,f'?lx{ext:#x}')
            fsf=(w>>21)&3; ftf=(w>>23)&3
            if n in ('DIV',): return f'DIV Q,vf{fs}{BC[fsf]},vf{ft}{BC[ftf]}'
            if n in ('SQRT','RSQRT'): return f'{n} Q,vf{ft}{BC[ftf]}' if n=='SQRT' else f'RSQRT Q,vf{fs}{BC[fsf]},vf{ft}{BC[ftf]}'
            if n=='MOVE' and d=='': return 'NOP'
            return f'{n}.{d} vf{ft},vf{fs}'
        return f'?low {op:#x}'
    n=LOW.get(top,f'?L{top:#x}'); imm11=w&0x7ff; imm11=imm11-0x800 if imm11&0x400 else imm11
    if n in ('B','BAL','IBEQ','IBNE','IBLTZ','IBGTZ','IBLEZ','IBGEZ'): return f'{n} vi{ft},vi{fs},{imm11:+d}'
    if n in ('FCSET','FCAND','FCOR','FCEQ'): return f'{n} {w&0xffffff:#x}'
    if n in ('FSAND','FSOR','FSEQ','FSSET','FMAND','FMOR','FMEQ'): return f'{n} vi{ft},{w&0xfff:#x} vi{fs}'
    if n in ('LQ','SQ','ILW','ISW'): return f'{n}.{d} vf{ft},{imm11}(vi{fs})'
    if n in ('IADDIU','ISUBIU'): return f'{n} vi{ft},vi{fs},{(w&0x7ff)|((w>>10)&0x7800)}'
    return f'{n} {w:#x}'
if __name__=='__main__':
    d=open(sys.argv[1],'rb').read(); pc=int(sys.argv[2],16)
    while True:
        lo,hi=struct.unpack_from('<II',d,pc*8)
        flags=''.join(f for f,b in (('I',63),('E',62),('M',61),('D',60),('T',59)) if (hi<<32)>>b&1)
        lw = f'LOI {struct.unpack("<f",struct.pack("<I",lo))[0]!r} ({lo:#010x})' if hi>>31&1 else lower(lo)
        print(f'{pc:#05x} [{flags:3}] {upper(hi):34s} | {lw}')
        if hi>>30&1:
            lo,hi=struct.unpack_from('<II',d,(pc+1)*8); print(f'{pc+1:#05x} (delay)  {upper(hi):34s} | {lower(lo) if not hi>>31&1 else "LOI"}'); break
        pc+=1
