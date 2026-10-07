# Ground bounce as decoded from the game's ball step; checked against captured first bounces.
import numpy as np, struct, glob, sys
import os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__))); from check_shots import step, load
f32=np.float32
COURT=dict(mu=0.3, relax=0.3, spin2vel=0.2, rest=0.7, couple=0.003)   # court 10 table entries
R=0.064
def unit(v): return v/np.sqrt((v*v).sum())
def bounce(v, spin, spin_frame, o, first=True, court=COURT, r=R):
    spin_side, spin_fwd = spin_frame[0], spin_frame[2]
    n=np.array([0,-1,0],f32)                       # ground normal (Y-down)
    if first and struct.unpack_from('<f',o,0x1a8)[0]>0: spin=f32(struct.unpack_from('<f',o,0x1a8)[0])
    old=spin
    vn=n*np.dot(v,n); vt=v-vn
    if first and struct.unpack_from('<f',o,0x1a8)[0]<0: spin=f32(struct.unpack_from('<f',o,0x1a8)[0])
    vtl=np.linalg.norm(vt)
    f=np.inf if vtl==0 else np.linalg.norm(vn)*court['mu']/vtl
    vt = vt*0 if f>=1 else vt-vt*f
    speed=np.linalg.norm(vt)
    up=unit(-n)
    if speed!=0: side=unit(np.cross(up,vt)); fwd=unit(np.cross(side,up))
    else: side, fwd = spin_frame[0], spin_frame[2]   # game keeps its previous contact basis
    s=np.array([np.dot(side,spin_side),np.dot(up,spin_side),np.dot(fwd,spin_side)])
    k=court['spin2vel']; sr=spin*r
    t=np.array([(-s[2])*sr*k, 0.0, (s[0]*sr-speed)*k+speed])
    vt2=t[0]*side+t[1]*up+t[2]*fwd
    fw_spin=np.dot(spin_fwd,vt2)   # projected on the ball's pre-bounce spin frame
    spin2=court['relax']*(fw_spin/r-spin)+spin
    e=court['rest']
    if first:
        cls,kind=o[0x58],struct.unpack_from('<i',o,0x5c)[0]
        if cls in (1,2): e*= {1:1.0,4:1.0}.get(kind,1.0)
        m=struct.unpack_from('<f',o,0x1ac)[0]
        if m!=0: e*=m
    vn2=vn*-e
    c=court['couple']
    if first and o[0x58] in (1,2): c*={1:0.5,4:0.1}.get(struct.unpack_from('<i',o,0x5c)[0],1.0)
    vn2=vn2+unit(vn2)*abs(spin2-old)*c
    spin2=spin2*(1-court['relax'])
    return vn2+vt2, spin2
if __name__=='__main__':
    for bf in sorted(glob.glob('shots_s05/shot_*.ball')):
        o,prm,X,V,cnt=load(bf[:-5]); fb=int(np.argmax(cnt>0)) if (cnt>0).any() else 0
        if fb<8 or fb+2>=len(V): continue
        spin=struct.unpack_from('<f',o,0x1a4)[0]; side=np.array(struct.unpack_from('<12f',o,0x160)).reshape(3,4)[:,:3]
        v=step(V[fb-1],spin,0.04,0.9)
        v2,s2=bounce(v.astype(np.float64),spin,side,o)
        nxt=V[fb+1]; print(bf[-12:-5], 'kind',o[0x58],struct.unpack_from('<i',o,0x5c)[0],'vel err', np.abs(v2-V[fb]).max().round(5), ' got',v2.round(4),' want',V[fb].round(4))
