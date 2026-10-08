"""P17s5: every VU1 draw record in an EE RAM dump (vtable 0x1d0e68: +4 light block, +0x10 object flags, +0x44 the
batch, whose packet records (vtable 0x1d11e8) hold +0xc = the packet's VIF in RAM), mapped to court 4's
(model, material) by finding each packet's VIF in RAM; prints (light block, its ambient/light/third, flags).
Flags bit 1 = VU1's unlit path. Run from the repo root: python3 research/p17s5_draws.py"""
import sys, struct, glob, collections
sys.path.insert(0, "research/tools"); import mdlwalk
ram=open('context/p17s2/s4.ram','rb').read()
def packets(path):
    d=open(path,'rb').read(); c=mdlwalk.C(d); c.take(1); mdlwalk.node(c,0,{'nodes':0}); out=[]
    for mi in range(c.u32(c.take(4))):
        for _ in range(c.u32(c.take(0xc))):
            bh=c.take(0x34)
            for _ in range(c.u32(bh,0x1c)):
                ph=c.take(0x60); vif=c.take(c.u32(ph,0x34)<<4)
                c.take(c.u32(ph,0x3c)+1); c.take(c.u32(ph,0x38)); c.take(c.u32(ph,0x38)<<5)
                if ph[0x56]: c.take(ph[0x57]<<4)
                for _ in range(struct.unpack_from('<h',ph,0x54)[0]):
                    sh=c.take(0xc); n=c.u32(sh)
                    if n: c.take(n<<4)
                if c.u32(ph,0x4c)<0: c.take(4)
                out.append((mi,ph,vif))
    return out
ram=open('context/p17s2/s4.ram','rb').read()
u=lambda a: struct.unpack_from('<I',ram,a)[0]
f=lambda a: struct.unpack_from('<f',ram,a)[0]
vifmap={}
for path in set(glob.glob('context/scratch_xb/COURT/04/*/data/court/course04/*/*.mdl')+glob.glob('context/scratch_xb/COURT/04/*/*/data/court/course04/*/*.mdl')):
    try: pk=packets(path)
    except Exception: continue
    for mi,ph,vif in pk:
        a=ram.find(vif[:96])
        while a>=0: vifmap[a]=(path.split('/')[-1],mi); a=ram.find(vif[:96],a+1)
res=collections.defaultdict(set)
for a in range(0x400000,0x2000000,16):
    if u(a)!=0x1d0e68: continue
    blk=u(a+4); flag=u(a+0x10); B=u(a+0x44)
    if not (0x100000<B<0x2000000): continue
    names=set()
    for o in range(0,0x60,4):
        p=u(B+o)
        if 0x100000<p<0x2000000 and u(p)==0x1d11e8:
            v=u(p+0xc)
            if v in vifmap: names.add(vifmap[v])
    # dive one more level
    if not names:
        for o in range(0,0x60,4):
            p=u(B+o)
            if 0x100000<p<0x2000000:
                for o2 in range(0,0x40,4):
                    q=u(p+o2)
                    if 0x100000<q<0x2000000 and u(q)==0x1d11e8 and u(q+0xc) in vifmap: names.add(vifmap[u(q+0xc)])
    fac=tuple(round(f(blk+0x70+4*i),4) for i in range(3)) if 0x100000<blk<0x2000000 else None
    for n in names or {('?',0)}: res[(n[0],n[1])].add((hex(blk),fac,hex(flag)))
for k in sorted(res): print(k, sorted(res[k]))
