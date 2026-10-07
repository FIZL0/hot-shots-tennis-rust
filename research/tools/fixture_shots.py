# Export each captured shot's first flight segment (frames 3..first bounce) as CSV:
# shot,frame,spin,px,py,pz,vx,vy,vz  (game Y-down). Spin = the constant spin solved from the path itself.
import sys, glob, numpy as np
import os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__))); from check_shots import load
def spin_of(v, vn, k=0.04, g=0.9, C=0.0015):
    s=np.linalg.norm(v); a=v-v*s*k; a[1]+=g*0.0027222224
    w=np.cross(v,np.cross([0,1,0],v)); w/=np.linalg.norm(w); return np.dot(vn-a,w)/(s*C)
with open(sys.argv[2],'w') as out:
    for si,bf in enumerate(sorted(glob.glob(sys.argv[1]+'/shot_*.ball'))):
        o,prm,X,V,cnt=load(bf[:-5])
        fb=int(np.argmax(cnt>0)) if (cnt>0).any() else len(V)
        if fb < 8: continue
        spins=np.array([spin_of(V[i],V[i+1]) for i in range(3,fb-1)]); spin=float(np.median(spins))
        if np.abs(spins-spin).max() > 1e-3: continue   # capture raced two shots together; unusable
        for i in range(3,fb):
            out.write(",".join(map(repr,[si,i,spin]+[float(x) for x in (*X[i],*V[i])]))+"\n")
