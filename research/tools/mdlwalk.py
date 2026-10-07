# Structural walk of .MDL as reconstructed from the game's loader; checks every file is consumed exactly.
import struct, sys, os
class C:
    def __init__(s,d): s.d=d; s.p=0
    def take(s,n,al=16):
        if s.p+n>len(s.d): raise EOFError(f"need {n} at {s.p:#x}/{len(s.d):#x}")
        b=s.d[s.p:s.p+n]; s.p+=n; s.p+=(-s.p)&(al-1); return b
    def u32(s,b,o=0): return struct.unpack_from('<i',b,o)[0]
def node(c, depth=0, st=None):
    h=c.take(0x40); c.take(c.u32(h,0x38)); c.take(0xc0)
    st['nodes']+=1
    n=c.u32(c.take(4))
    for _ in range(n):
        h=c.take(0x40); c.take(c.u32(h,0x38)); c.take(0xc0); st['nodes']+=1
        k=c.u32(c.take(4))
        for _ in range(k): node(c, depth+1, st)
def mdl(d):
    c=C(d); st={'nodes':0,'mats':0,'batches':0,'packets':0,'verts':0}
    c.take(1); node(c,0,st)
    nm=c.u32(c.take(4)); st['mats']=nm
    for _ in range(nm):
        nb=c.u32(c.take(0xc))
        for _ in range(nb):
            bh=c.take(0x34); st['batches']+=1
            for _ in range(c.u32(bh,0x1c)):
                ph=c.take(0x60); st['packets']+=1
                c.take(c.u32(ph,0x34)<<4); c.take(c.u32(ph,0x3c)+1); c.take(c.u32(ph,0x38)); c.take(c.u32(ph,0x38)<<5)
                if ph[0x56]: c.take(ph[0x57]<<4)
                for _ in range(struct.unpack_from('<h',ph,0x54)[0]):
                    sh=c.take(0xc); n=c.u32(sh)
                    if n: c.take(n<<4)
                if c.u32(ph,0x4c)<0: c.take(4)
    nt=c.u32(c.take(4))
    for _ in range(nt): c.take(c.u32(c.take(4)))
    return c.p, st
if __name__=='__main__':
    ok=bad=0
    for r,_,fs in os.walk(sys.argv[1]):
        for f in fs:
            if f.lower().endswith('.mdl'):
                p=os.path.join(r,f); d=open(p,'rb').read()
                try:
                    end,st=mdl(d)
                    if end==len(d) or (end<len(d) and not any(d[end:])): ok+=1
                    else: bad+=1; print('LEFTOVER',p,hex(end),hex(len(d))) if bad<10 else None
                except Exception as e:
                    bad+=1; print('ERR',p,e) if bad<10 else None
    print(ok,'ok',bad,'bad')
