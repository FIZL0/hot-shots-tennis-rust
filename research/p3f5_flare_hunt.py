# P3f5: free-run from a save slot; when the sun's lens flare shows (its view cosine > lim, fade > 0.5) and holds over a vsync, pause, F8 + FrameAdvance, keep
# the shot if the view stayed put across it. Frame-exact shot recipe from R1/B63 (F8, then FrameAdvance).
# Run under tools/pcsx2.sh from the repo root; shots go to context/p3f5/<tag>_<n>.png. usage: p3f5_flare_hunt.py slot secs tag [lim] [max]
import sys,struct,time,subprocess,os
sys.path.insert(0,'tools')
os.environ['HST_LOCKSTEP']='1'
from pine import Pine, VSYNC
p=Pine(step=True)
slot,secs,tag=int(sys.argv[1]),float(sys.argv[2]),sys.argv[3]
lim=float(sys.argv[4]) if len(sys.argv)>4 else 0.2; mx=int(sys.argv[5]) if len(sys.argv)>5 else 6
p.lockstep=False; p.load_state(slot); time.sleep(3)
fl=lambda a:struct.unpack('<f',struct.pack('<I',p._u32(a)))[0]
def flare():
    f=p._u32(p._u32(p._u32(0x422f80)+0x84)+0x15c)
    return fl(f+0x9d0), fl(0x1e7c80)
# P3F5_POKE="a,b": each poll, point the flare's sun (not the court's light) along the view's forward + a·right + b·up
poke=[float(x) for x in os.environ['P3F5_POKE'].split(',')] if os.environ.get('P3F5_POKE') else None
def sun_dir():
    f=p._u32(p._u32(p._u32(0x422f80)+0x84)+0x15c)
    return [fl(f+0x10+4*i) for i in range(3)]
def poke_sun(v):
    f=p._u32(p._u32(p._u32(0x422f80)+0x84)+0x15c)
    fw,dn=v[3:6],v[6:9]
    r=[fw[1]*dn[2]-fw[2]*dn[1],fw[2]*dn[0]-fw[0]*dn[2],fw[0]*dn[1]-fw[1]*dn[0]]
    d=[fw[i]+poke[0]*r[i]-poke[1]*dn[i] for i in range(3)]
    n=sum(x*x for x in d)**0.5
    for i in range(3): p.write32(f+0x10+4*i,struct.unpack('<I',struct.pack('<f',d[i]/n))[0])
def view(): return [fl(0x1e7d20+4*i) for i in range(3)]+[fl(0x1e7d30+4*i) for i in range(3)]+[fl(0x1e7d40+4*i) for i in range(3)]+[fl(0x1e7d64)]
d=f"/home/ryha/repos/HST-Remastered-slots/pcsx2/s{os.environ['HST_PCSX2']}/PCSX2/snaps"
odir=os.environ.get("HST_SHOTS","context/p3f5"); os.makedirs(odir,exist_ok=True)
pid=open(f"{os.environ['XDG_RUNTIME_DIR']}/hst-pcsx2{os.environ['HST_PCSX2']}.pid").read().strip()
t0=time.time();n=0;prev=None;last_shot=-999
while time.time()-t0<secs and n<mx:
    v=view(); vs=p._u32(VSYNC)
    if poke: poke_sun(v)
    c,fade=flare()
    if c>lim and fade>0.5 and prev and prev[0]!=vs and (poke or prev[1]==v) and vs-last_shot>(600 if poke else 120):
        p.lockstep=True; p.pause(); v1=view(); vs1=p._u32(VSYNC); cf1=flare()
        before=set(os.listdir(d))
        # the copies bind Screenshot to L3 + R3 on the pad only
        subprocess.run(['tools/vpad.py','send','down l3','down r3','sleep 120','up r3','up l3'],capture_output=True)
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
            print(n,vs1,'c %.3f fade %.3f'%cf1,'then %.3f %.3f'%flare(),'held' if v1==v2==v else 'MOVED','sun',','.join('%.9g'%x for x in sun_dir()),','.join('%.9g'%x for x in v1),flush=True); n+=1
        else: print('no png',flush=True)
        last_shot=vs1; p.lockstep=False; p.resume(); prev=None; continue
    if not prev or prev[0]!=vs: prev=(vs,v)
print('court',p._u32(0x422f90))
