# B74b: free-run from a save slot; when the main view looks up (fy < lim) and holds over a vsync, pause, F8 + FrameAdvance, keep
# the shot if the view stayed put across it. Frame-exact shot recipe from R1/B63 (F8, then FrameAdvance).
# Run under tools/pcsx2.sh from the repo root; shots go to context/shots/b74b/<tag>_<n>.png. usage: b74b_hunt.py slot secs tag [lim] [max]
import sys,struct,time,subprocess,os
sys.path.insert(0,'tools')
os.environ['HST_LOCKSTEP']='1'
from pine import Pine, VSYNC
p=Pine(step=True)
slot,secs,tag=int(sys.argv[1]),float(sys.argv[2]),sys.argv[3]
lim=float(sys.argv[4]) if len(sys.argv)>4 else 0.1; mx=int(sys.argv[5]) if len(sys.argv)>5 else 6
p.lockstep=False; p.load_state(slot); time.sleep(3)
fl=lambda a:struct.unpack('<f',struct.pack('<I',p._u32(a)))[0]
def view(): return [fl(0x1e7d20+4*i) for i in range(3)]+[fl(0x1e7d30+4*i) for i in range(3)]+[fl(0x1e7d40+4*i) for i in range(3)]+[fl(0x1e7d64)]
d=f"/home/ryha/repos/opentennis-slots/pcsx2/s{os.environ['HST_PCSX2']}/PCSX2/snaps"
odir=os.environ.get("HST_SHOTS","context/shots/b74b"); os.makedirs(odir,exist_ok=True)
pid=open(f"{os.environ['XDG_RUNTIME_DIR']}/hst-pcsx2{os.environ['HST_PCSX2']}.pid").read().strip()
t0=time.time();n=0;prev=None;last_shot=-999
while time.time()-t0<secs and n<mx:
    v=view(); vs=p._u32(VSYNC)
    if v[4]<lim and prev and prev[0]!=vs and prev[1]==v and vs-last_shot>120:
        p.lockstep=True; p.pause(); v1=view(); vs1=p._u32(VSYNC)
        before=set(os.listdir(d))
        subprocess.run(['hyprctl','dispatch',f'hl.dsp.send_shortcut({{ mods = "", key = "F8", window = "pid:{pid}" }})'],capture_output=True)
        p.next_frame(vs1); v2=view()
        new=set()
        for _ in range(40):
            new=set(os.listdir(d))-before
            if new: break
            time.sleep(0.2)
        if new:
            f=os.path.join(d,new.pop())
            for _ in range(30):
                s=os.path.getsize(f); time.sleep(0.3)
                if s>0 and s==os.path.getsize(f): break
            out=f'{odir}/{tag}_{n}.png'; subprocess.run(['cp',f,out])
            print(n,vs1,'held' if v1==v2==v else 'MOVED',','.join('%.9g'%x for x in v1),flush=True); n+=1
        else: print('no png',flush=True)
        last_shot=vs1; p.lockstep=False; p.resume(); prev=None; continue
    if not prev or prev[0]!=vs: prev=(vs,v)
print('court',p._u32(0x422f90))
