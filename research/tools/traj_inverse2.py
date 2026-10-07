# 2-D inverse: find a target point whose table lookup reproduces launch speed+elevation exactly, then compare the
# yaw between launch direction and target direction against spin × constant.
import sys, glob, struct, numpy as np
import os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__))); from traj_model import lookup, f32
from traj_inverse import coords, tables
from scipy.optimize import least_squares
tabs={n:np.frombuffer(d,'<u4') for d,n in tables.items() if 'strk' in n}
for bf in sorted(glob.glob(sys.argv[1]+'/shot_*.ball')):
    b=open(bf,'rb').read(); fr=struct.unpack_from('<I',b,0)[0]; o=b[4:4+0x290]
    if fr!=0 or o[0x58]!=1: continue
    kind=struct.unpack_from('<i',o,0x5c)[0]
    pos=np.array(struct.unpack_from('<3f',o,0xe0)); v=np.array(struct.unpack_from('<3f',o,0x130))
    speed=np.linalg.norm(v); h=np.hypot(v[0],v[2]); elev=np.arctan2(-v[1],h)
    if speed<0.15: continue
    spin=struct.unpack_from('<f',o,0x1a4)[0]; frames=struct.unpack_from('<i',o,0x260)[0]
    best=None
    for name,t in tabs.items():
        def f(p):
            tgt=np.array([p[0],0,p[1]]); c=coords(pos,tgt,1,kind)
            e,s,n=lookup(t,c[1],c[0],c[3],c[2],c[5],c[4]); return [float(e)-elev,(float(s)-speed)*10]
        land=pos+np.array([v[0],0,v[2]])/h*20
        for guess in ([land[0],land[2]],[pos[0]+v[0]/h*15,pos[2]+v[2]/h*15]):
            r=least_squares(f,guess,diff_step=1e-3)
            err=np.abs(r.fun).sum()
            c=coords(pos,np.array([r.x[0],0,r.x[1]]),1,kind); n=lookup(t,c[1],c[0],c[3],c[2],c[5],c[4])[2]
            if n+1!=frames: err+=1.0   # flight frames must agree too (frames = table + 1)
            if best is None or err<best[0]: best=(err,name,r.x)
    err,name,(tx,tz)=best
    td=np.arctan2(tx-pos[0],tz-pos[2]); vd=np.arctan2(v[0],v[2]); yaw=(vd-td+np.pi)%(2*np.pi)-np.pi
    c=coords(pos,np.array([tx,0,tz]),1,kind); e,s,n=lookup(tabs[name],c[1],c[0],c[3],c[2],c[5],c[4])
    print(f"{bf[-8:-5]} kind {kind} spin {spin:+.4f} err {err:.1e} {name:22s} target ({float(tx)!r},{float(tz)!r}) frames {frames} tbl {n}  yaw {np.degrees(yaw):+7.3f}°  yaw/spin {yaw/spin if spin else 0:+.4f}")
