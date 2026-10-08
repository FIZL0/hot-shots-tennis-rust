"""P17s5: a static model's VU1 light in a GS dump, with a packet matcher that survives repeated kick patterns.
Each packet takes the GS run (same kick pattern, back faces culled) whose GS/vc ratio is most constant over
vertices of equal normal; runs and packets are paired greedily, each at most once. Then per node (K=0), packet
(K=4) or batch group (K=5), it fits a rotation R (or uses the node matrix given as R=r00,..,r22, row vectors
n·R, as in RAM) and prints how many vertices match
  GS = ⌊vc · mat · (A + L·max(−Rn·l1, 0) + T·max(−Rn·l2, 0))⌋.

  p17s5_check.py dump.gs model.txt A L T l1x l1y l1z l2x l2y l2z   (model.txt: `noidump model.mdl model.mtl`)"""
import sys, os, collections
import numpy as np
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from p17s_light_gs import stream
from scipy.optimize import least_squares
from scipy.spatial.transform import Rotation
dump, model = sys.argv[1], sys.argv[2]
A,L,T = map(float, sys.argv[3:6]); l1=np.array(sys.argv[6:9],float); l2=np.array(sys.argv[9:12],float)
runs = stream(open(dump,'rb').read())
mats, pks = {}, []
for line in open(model):
    t=line.split()
    if t[0]=='M': mats[int(t[1])]=[float(x) for x in t[2:5]]
    elif t[0]=='P': pks.append(dict(material=int(t[1]),group=int(t[2]),verts=[]))
    elif t[0]=='V': pks[-1]['verts'].append([int(t[1]),[int(x) for x in t[2:5]],[]])
    elif t[0]=='E': pks[-1]['verts'][-1][2].append((int(t[5]),tuple(float(x) for x in t[7:10])))
def spread(pk, r):
    g=collections.defaultdict(list)
    for (_,vc,es),v in zip(pk['verts'],r):
        if vc[0]>=16: g[es[0][1]].append(v[3][0]/vc[0])
    return sum(max(x)-min(x) for x in g.values()), len(g)
rows=[]; used=collections.Counter(); allc=[]
for i,pk in enumerate(pks):
    sig=[v[0] for v in pk['verts']]
    if len(sig)<6: continue
    for r in {tuple(r) for prim,r in runs if prim&0x10 and len(r)==len(sig) and all(v[2]<=k for v,k in zip(r,sig)) and sum(v[2] for v in r)*2>sum(sig)}:
        allc.append((spread(pk,r)[0]/len(sig),i,r))
pdone,rdone=set(),set()
for sp,i,r in sorted(allc,key=lambda x:x[0]):
    if i in pdone or r in rdone or sp>0.05: continue
    pdone.add(i); rdone.add(r); pk=pks[i]; m=mats[pk['material']]
    for (_,vc,es),g in zip(pk['verts'],r):
        if len(es)==1: rows.append((es[0][0],np.array(es[0][1]),np.array(vc)*m,g[3][:3],i,pk['group']))
print(len(rows),'rows; runs reused', sum(1 for v in used.values() if v>1))
import os
K=int(os.environ.get('K','0'))
for node in sorted({r[K] for r in rows}):
    rs=[r for r in rows if r[K]==node]
    n=np.array([r[1] for r in rs]); vm=np.array([r[2] for r in rs]); g=np.array([r[3] for r in rs],float)
    def pred(rv):
        rn=Rotation.from_rotvec(rv).apply(n)
        k=A+L*np.maximum(-rn@l1,0)+T*np.maximum(-rn@l2,0)
        return vm*k[:,None]
    if os.environ.get('R'):
        Rm=np.array([float(x) for x in os.environ['R'].split(',')]).reshape(3,3); rn=n@Rm
        err=np.floor(vm*(A+L*np.maximum(-rn@l1,0)+T*np.maximum(-rn@l2,0))[:,None])-g
        print('node',node,'exact',int((np.abs(err).max(1)==0).sum()),'/',len(rs),'max',np.abs(err).max()); continue
    best=min((least_squares(lambda rv:(pred(rv)-g-0.5).ravel(),x0) for x0 in Rotation.random(12,random_state=1).as_rotvec()),key=lambda r:r.cost)
    err=np.floor(pred(best.x))-g
    bad=sorted(collections.Counter(r[4] for r,e in zip(rs,np.abs(err).max(1)) if e>1).items()); print('bad pks',bad)
    print('node',node,'exact',int((np.abs(err).max(1)==0).sum()),'/',len(rs),'max',np.abs(err).max(), 'rot', np.round(Rotation.from_rotvec(best.x).as_euler('xyz',degrees=True),1))
