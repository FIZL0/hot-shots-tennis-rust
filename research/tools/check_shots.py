# For each captured shot: replay the flight model from path[0] with the shot's own params; report exactness to first bounce.
import struct, glob, sys, numpy as np
f32=np.float32
def step(v,spin,k,g,C=0.0015,axis=(0,1,0)):
    v=v.astype(f32); s=np.sqrt(f32((v*v).sum()))
    if s==0: return v
    inv=f32(1)/s; v=v-v*inv*s*s*f32(k)
    a=np.array(axis,f32); c=np.cross(a,v).astype(f32); w=np.cross(v,c).astype(f32); wl=f32(1)/np.sqrt(f32((w*w).sum()))
    v=(w*wl*s*f32(spin)*f32(C)+v).astype(f32); v[1]+=f32(g)*f32(0.0027222224); return v
def load(base):
    b=open(base+'.ball','rb').read(); o=b[4:4+0x290]; prm=b[4+0x290:]; raw=open(base+'.path','rb').read()
    P=np.frombuffer(raw,'<f4').reshape(-1,12).astype(np.float64); cnt=np.frombuffer(raw,'<u4').reshape(-1,12)[:,8]
    V=P[:,4:7].copy(); V[:,1]*=-1; X=P[:,:3].copy(); X[:,1]*=-1
    return o,prm,X,V,cnt
if __name__=='__main__':
    ok=tot=0
    for bf in sorted(glob.glob(sys.argv[1]+'/shot_*.ball')):
        base=bf[:-5]; o,prm,X,V,cnt=load(base)
        fl=lambda off: struct.unpack_from('<f',o,off)[0]
        k,g=struct.unpack_from('<f',prm,0x8c4)[0],struct.unpack_from('<f',prm,0x8c0)[0]
        fb=int(np.argmax(cnt>0)) if (cnt>0).any() else len(V)
        if fb<3: continue
        e=max(np.abs(step(V[i],fl(0x1a4),k,g)-V[i+1]).max() for i in range(fb-1))
        tot+=1; ok+=e<2e-5
        print(f"{base[-8:]} n={len(V):3d} bounce@{fb:3d} spin={fl(0x1a4):+.4f} type58={o[0x58]} 5c={struct.unpack_from('<i',o,0x5c)[0]} curve254={fl(0x254):+.3f} err={e:.1e}")
    print(ok,'/',tot,'shots reproduce exactly up to first bounce')
