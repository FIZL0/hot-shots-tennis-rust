# B74d: free-run a save slot until the court view's eye is within `tol` m of a target, then take PCSX2's F11 one-frame
# GS dump and an F8 screenshot (no pause: F8 fails while paused, lock-step is gone) and print the view read before
# and after, with the vsync. Run under tools/pcsx2.sh. usage: b74d_grab.py slot secs out_dir ex ey ez [tol]
import sys,struct,time,subprocess,os,shutil
sys.path.insert(0,'tools')
from pine import Pine, VSYNC
p=Pine()
slot,secs,odir=int(sys.argv[1]),float(sys.argv[2]),sys.argv[3]
tgt=[float(x) for x in sys.argv[4:7]]; tol=float(sys.argv[7]) if len(sys.argv)>7 else 0.05
os.makedirs(odir,exist_ok=True)
fl=lambda a:struct.unpack('<f',struct.pack('<I',p._u32(a)))[0]
def view(): return [fl(0x1e7d20+4*i) for i in range(3)]+[fl(0x1e7d30+4*i) for i in range(3)]+[fl(0x1e7d40+4*i) for i in range(3)]+[fl(0x1e7d64)]
d=f"{os.path.dirname(os.path.abspath('.'))}/pcsx2/s{os.environ['HST_PCSX2']}/PCSX2/snaps"; os.makedirs(d,exist_ok=True)
pid=open(f"{os.environ['XDG_RUNTIME_DIR']}/hst-pcsx2{os.environ['HST_PCSX2']}.pid").read().strip()
key=lambda k:subprocess.run(['hyprctl','dispatch',f'hl.dsp.send_shortcut({{ mods = "", key = "{k}", window = "pid:{pid}" }})'],capture_output=True)
p.load_state(slot); time.sleep(3); t0=time.time()
while time.time()-t0<secs:
    v=view()
    if max(abs(v[i]-tgt[i]) for i in range(3))<tol: break
else: sys.exit('target not reached')
vs0=p._u32(VSYNC); before=set(os.listdir(d))
key('F11'); key('F8'); v1=view(); vs1=p._u32(VSYNC)
print('vsync',vs0,vs1); print('view0',','.join('%.9g'%x for x in v)); print('view1',','.join('%.9g'%x for x in v1),flush=True)
time.sleep(6)
for f in sorted(set(os.listdir(d))-before):
    shutil.copy(os.path.join(d,f),os.path.join(odir,f.replace(' ','_'))); print('got',f)
