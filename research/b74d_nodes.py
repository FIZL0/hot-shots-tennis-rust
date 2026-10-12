# B74d: free-run a save slot until the court view's eye is within `tol` m of a target, pause, dump EE RAM to `out`
# (read in 64 KB chunks) and resume. The model nodes in the dump (stride 0x120) hold the per-node frustum state: centre
# +0xc0, state u16 +0x110 (0x100 drawn, 0x200 culled, 0x300 drawn through VU1's clip pass), class byte +0x112.
# Run under tools/pcsx2.sh. usage: b74d_nodes.py slot secs out ex ey ez [tol]
import sys,struct,time,os
sys.path.insert(0,'tools')
from pine import Pine, VSYNC
p=Pine()
slot,secs,out=int(sys.argv[1]),float(sys.argv[2]),sys.argv[3]
tgt=[float(x) for x in sys.argv[4:7]]; tol=float(sys.argv[7]) if len(sys.argv)>7 else 0.05
fl=lambda a:struct.unpack('<f',struct.pack('<I',p._u32(a)))[0]
view=lambda:[fl(0x1e7d20+4*i) for i in range(3)]+[fl(0x1e7d30+4*i) for i in range(3)]
p.load_state(slot); time.sleep(3); t0=time.time()
while time.time()-t0<secs:
    if max(abs(a-b) for a,b in zip(view(),tgt))<tol: break
else: sys.exit('target not reached')
p.pause(); v=view(); vs=p._u32(VSYNC)
ram=b''.join(p.read_block(a,0x10000) for a in range(0,0x2000000,0x10000))
p.resume(); open(out,'wb').write(ram)
print('vsync',vs,'view',','.join('%.9g'%x for x in v),flush=True)
