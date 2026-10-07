import os, sys, struct, collections
sys.path.insert(0, os.path.dirname(__file__))
import mdlwalk, vifwalk
# Collect VIF unpack signatures per packet across all models.
sigs=collections.Counter(); ex={}
orig=mdlwalk.C.take
def run(p):
    d=open(p,'rb').read(); pk=[]
    def take(s,n,al=16):
        b=orig(s,n,al); return b
    c=mdlwalk.C(d)
    # re-walk but capture vif blocks: replicate packet loop
    c.take(1); mdlwalk.node(c,0,{'nodes':0})
    for _ in range(c.u32(c.take(4))):
        for _ in range(c.u32(c.take(0xc))):
            bh=c.take(0x34)
            for _ in range(c.u32(bh,0x1c)):
                ph=c.take(0x60); vif=c.take(c.u32(ph,0x34)<<4)
                a=c.take(c.u32(ph,0x3c)+1); b=c.take(c.u32(ph,0x38)); m=c.take(c.u32(ph,0x38)<<5)
                if ph[0x56]: c.take(ph[0x57]<<4)
                for _ in range(struct.unpack_from('<h',ph,0x54)[0]):
                    sh=c.take(0xc); n=c.u32(sh)
                    if n: c.take(n<<4)
                if c.u32(ph,0x4c)<0: c.take(4)
                cmds=[t for _,t,_ in vifwalk.walk(vif,0,10000) if t.startswith('UNPACK') or t.startswith('??') or t.startswith('DIRECT') or t.startswith('MSC')]
                sig=' | '.join(x.split(' addr')[0].replace('UNPACK ','') + ('' if 'n=' not in x else '') for x in cmds)
                sig=' | '.join(sorted(set(sig.split(' | '))))
                key=(sig, c.u32(ph,0x38)>0)
                sigs[key]+=1; ex.setdefault(key,(p,ph.hex()))
for r,_,fs in os.walk(sys.argv[1]):
    for f in fs:
        if f.lower().endswith('.mdl'): run(os.path.join(r,f))
for k,v in sigs.most_common(30): print(v,k,ex[k][0])
