import os, sys, struct
sys.path.insert(0, os.path.dirname(__file__)); from mdlwalk import C
def tex(c, h, src):
    for i in range(7):
        n=struct.unpack_from('<i',h,0x10+i*8)[0]
        if n: src.take(n)
    k=struct.unpack_from('<h',h,0)[0]
    if k: src.take(k*4)
def mtl(d, mti):
    c=C(d); m=C(mti) if mti is not None else None
    nmat=c.u32(c.take(4))
    for _ in range(c.u32(c.take(4))):
        h=c.take(0x48)
        if m: tex(c,h,m)
    for _ in range(c.u32(c.take(4))):
        h=c.take(0x48); tex(c,h,c); c.take(0x10)
    for _ in range(nmat):
        h=c.take(0x30); x=struct.unpack_from('<h',h,0x1c)[0]
        if x: c.take(x)
    return c.p, (m.p if m else 0)
ok=bad=0
for r,_,fs in os.walk(sys.argv[1]):
    for f in fs:
        if f.lower().endswith('.mtl'):
            p=os.path.join(r,f); d=open(p,'rb').read(); b=p[:-4]
            mp=next((b+e for e in ('.MTI','.mti','.Mti') if os.path.exists(b+e)),None)
            mti=open(mp,'rb').read() if mp else None
            try:
                a,bb=mtl(d,mti)
                good = a<=len(d) and not any(d[a:]) and (mti is None or (bb<=len(mti) and not any(mti[bb:])))
                if good: ok+=1
                else:
                    bad+=1
                    if bad<8: print('LEFT',p,hex(a),hex(len(d)),hex(bb),hex(len(mti or b'')))
            except Exception as e:
                bad+=1
                if bad<8: print('ERR',p,e)
print(ok,'ok',bad,'bad')
