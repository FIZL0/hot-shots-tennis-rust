# Export whole captured shots for full-flight replay: lines
#   S,shot,court,start_frame,spin,curve,bend,curve_frames,first_spin,first_rest,class,kind,side3,wind3,spinframe9,contact9
#   P,shot,frame,px,py,pz,vx,vy,vz            (game Y-down; frames from start_frame to end of path)
import sys, glob, struct, numpy as np
import os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__))); from check_shots import load
def spin_of(v,vn,k=0.04,g=0.9,C=0.0015):
    s=np.linalg.norm(v); a=v-v*s*k; a[1]+=g*0.0027222224
    w=np.cross(v,np.cross([0,1,0],v)); w/=np.linalg.norm(w); return np.dot(vn-a,w)/(s*C)
court=int(sys.argv[3]); START=3
with open(sys.argv[2],'w') as out:
    for si,bf in enumerate(sorted(glob.glob(sys.argv[1]+'/shot_*.ball'))):
        o,prm,X,V,cnt=load(bf[:-5]); fb=int(np.argmax(cnt>0)) if (cnt>0).any() else len(V)
        if fb<8: continue
        spins=np.array([spin_of(V[i],V[i+1]) for i in range(START,fb-1)]); spin=float(np.median(spins))
        if np.abs(spins-spin).max()>1e-3: continue          # raced capture
        if abs(spin-struct.unpack_from('<f',o,0x1a4)[0])>1e-3: continue   # captured state belongs to another shot
        f=lambda off,n=1: list(struct.unpack_from(f'<{n}f',o,off))
        row=['S',si,court,START,repr(float(struct.unpack_from('<f',o,0x1a4)[0]))]+f(0x254)+f(0x250)+[struct.unpack_from('<i',o,0x260)[0]]+f(0x1a8)+f(0x1ac)+[o[0x58],struct.unpack_from('<i',o,0x5c)[0]]
        row+=f(0x90,3)+f(0x240,3)
        row+=sum((f(0x160+16*i,3) for i in range(3)),[])+sum((f(0x1c0+16*i,3) for i in range(3)),[])
        out.write(",".join(map(str,row))+"\n")
        for i in range(START,len(V)):
            out.write(",".join(['P',str(si),str(i)]+[repr(float(x)) for x in (*X[i],*V[i])])+"\n")
