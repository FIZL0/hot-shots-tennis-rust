import sys, glob, struct, numpy as np
import os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__))); from traj_model import load_table, lookup, norm, f32
from check_shots import load
ram=open('ram/s05.bin','rb').read()
tables={}
for f in sorted(glob.glob('xb/TRAJ/*/data/hatsuyama/traj/*.dat')):
    d=open(f,'rb').read()
    if ram.find(d)>=0: tables.setdefault(d, f.split('/')[-1][:-4])
tabs=[(n,np.frombuffer(d,'<u4')) for d,n in tables.items() if 'strk' in n]
H_CHAR=1.1638  # character height term (RAM 0x2f10fc) for the slot-5 player
def coords(hit, tgt, cls, kind):
    hit=np.array(hit,dtype=np.float32); tgt=np.array(tgt,dtype=np.float32)
    if hit[2]>0: hit=hit*[-1,1,-1]; tgt=tgt*[-1,1,-1]
    tgt=tgt.copy(); tgt[2]+=f32(-0.15)
    dz=tgt[2]-hit[2]; dist=np.sqrt(dz*dz+(tgt[0]-hit[0])**2)
    v7=dist*tgt[2]/dz
    ix,fx=norm(dist-v7+f32(-0.5), f32(-0.5)-f32(-18.17))
    lo=lerp_h(hit,cls,kind); hi=f32(-H_CHAR-1.3*1.3-0.5)
    iy,fy=norm(hit[1]-lo, hi-lo)
    z0,z1=(3.0,16.17) if kind not in (2,) else (6.4,16.17)
    iz,fz=norm(v7-f32(z0), f32(z1-z0))
    return ix,fx,iy,fy,iz,fz
def lerp_h(hit,cls,kind):
    u=(11.385-(abs(float(hit[2]))-0.5))/11.385; u=min(max(u,0),1)
    return f32(u*(0.0-(-0.2))+(-0.2))
res=[]
for bf in sorted(glob.glob('shots_s05/shot_*.ball')):
    b=open(bf,'rb').read(); fr=struct.unpack_from('<I',b,0)[0]; o=b[4:4+0x290]
    if fr!=0 or o[0x58]!=1: continue
    kind=struct.unpack_from('<i',o,0x5c)[0]
    pos=np.array(struct.unpack_from('<3f',o,0xe0)); v=np.array(struct.unpack_from('<3f',o,0x130))
    speed=np.linalg.norm(v); h=np.hypot(v[0],v[2]); elev=np.arctan2(-v[1],h); dirh=np.array([v[0],0,v[2]])/h
    frames=struct.unpack_from('<i',o,0x260)[0]
    best=None
    def scan(t, ds):
        out=None
        for dd in ds:
            tgt=pos+dirh*dd; tgt[1]=0
            c=coords(pos,tgt,1,kind)
            e,s,n=lookup(t,c[1],c[0],c[3],c[2],c[5],c[4])
            err=abs(float(e)-elev)+abs(float(s)-speed)
            if out is None or err<out[0]: out=(err,dd,float(e),float(s),n)
        return out
    for name,t in tabs:
        c=scan(t, np.arange(2.0,30.0,0.25))
        c=scan(t, np.arange(c[1]-0.3,c[1]+0.3,0.002))
        if best is None or c[0]<best[0]: best=(c[0],name,c[1],c[2],c[3],c[4])
    res.append(best)
    print(bf[-12:-5],'kind',kind,'speed',round(speed,4),'elev',round(np.degrees(elev),2),'frames',frames,'| best',best[1],'dist',round(best[2],2),'err',round(best[0],5),'tbl speed',round(best[4],4),'elev',round(np.degrees(best[3]),2),'frames',best[5])
