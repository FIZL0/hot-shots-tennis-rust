# Scan a buffer for VIF code streams (heuristic start at 0x01000404 STCYCL) and print commands.
import struct, sys
VN=['S','V2','V3','V4']; VL=['32','16','8','5']
def walk(d, start, limit=200, quiet=False):
    p=start; out=[]; cl=wl=4
    while p+4<=len(d) and len(out)<limit:
        w,=struct.unpack_from('<I',d,p); cmd=(w>>24)&0x7f; num=(w>>16)&0xff; imm=w&0xffff; p+=4
        if cmd>=0x60:
            vn,vl=(cmd>>2)&3,cmd&3; n=num or 256; n=n if cl>=wl else (n//wl)*cl+min(n%wl,cl); bpe=[4,2,1,2][vl]*(vn+1) if vl!=3 else 2
            size=((n*bpe+3)//4)*4
            out.append((p-4,f"UNPACK {VN[vn]}-{VL[vl]}{' m' if cmd&0x10 else ''} n={n} addr={imm&0x3ff:#x}{' usn' if imm&0x4000 else ''}{' tops' if imm&0x8000 else ''}", d[p:p+size]))
            p+=size
        elif cmd==0x00: out.append((p-4,'NOP',b''))
        elif cmd==0x01: cl,wl=imm&0xff,imm>>8; out.append((p-4,f'STCYCL cl={imm&0xff} wl={imm>>8}',b''))
        elif cmd==0x20: out.append((p-4,'STMASK',d[p:p+4])); p+=4
        elif cmd==0x30: out.append((p-4,'STROW',d[p:p+16])); p+=16
        elif cmd==0x31: out.append((p-4,'STCOL',d[p:p+16])); p+=16
        elif cmd in(0x14,0x15,0x17): out.append((p-4,f'MSCAL {imm:#x}' if cmd==0x14 else ('MSCALF' if cmd==0x15 else 'MSCNT'),b'')); 
        elif cmd==0x10: out.append((p-4,'FLUSHE',b''))
        elif cmd in (0x11,0x13): out.append((p-4,'FLUSH',b''))
        elif cmd==0x05: out.append((p-4,f'STMOD {imm}',b''))
        elif cmd==0x02: out.append((p-4,f'OFFSET',b''))
        elif cmd==0x03: out.append((p-4,f'BASE',b''))
        elif cmd==0x04: out.append((p-4,f'ITOP {imm}',b''))
        elif cmd==0x50 or cmd==0x51: n=imm*16; out.append((p-4,f'DIRECT qw={imm}',d[p:p+n])); p+=n
        else: out.append((p-4,f'?? {w:08x}',b'')); break
    return out
if __name__=='__main__':
    d=open(sys.argv[1],'rb').read(); s=d.find(struct.pack('<I',0x01000404))
    for off,txt,data in walk(d,s,int(sys.argv[2]) if len(sys.argv)>2 else 200):
        print(f"{off:06x} {txt}  {data[:32].hex(' ',4)}")
