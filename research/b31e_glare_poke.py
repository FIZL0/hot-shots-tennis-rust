# B31e: force the sun glare in the original (slot 5) for a by-eye check: point the light at the camera (d90 = -forward)
# and raise the look-up clamp floor to 0.8, F8 a frame with it off / on / off. Shots: context/b31e/lk_*.png.
# Run under tools/pcsx2.sh from the repo root.
import sys,struct,time,subprocess,os
sys.path.insert(0,'tools')
os.environ['HST_LOCKSTEP']='1'
from pine import Pine, VSYNC
p=Pine(step=True)
p.lockstep=False; p.load_state(5); time.sleep(5)
fl=lambda a:struct.unpack('<f',struct.pack('<I',p._u32(a)))[0]
F=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
d=f"/home/ryha/repos/open-tennis-slots/pcsx2/s{os.environ['HST_PCSX2']}/PCSX2/snaps"
p.lockstep=True; p.pause()
for tag,v in [('lk_off',0.0),('lk_on',0.8),('lk_off2',0.0)]:
    fw=[fl(0x1e7f50+4*i) for i in range(3)]
    for i in range(3): p.write32(0x1e7d90+4*i,F(-fw[i]))
    p.write32(0x1bb660,F(v))
    before=set(os.listdir(d))
    pid=open(os.environ['XDG_RUNTIME_DIR']+'/hst-pcsx2'+os.environ['HST_PCSX2']+'.pid').read().strip(); subprocess.run(['hyprctl','dispatch','hl.dsp.send_shortcut({ mods = "", key = "F8", window = "pid:'+pid+'" })'],capture_output=True)
    p.next_frame(p._u32(VSYNC)); time.sleep(0.5); p.next_frame(p._u32(VSYNC))
    new=set()
    for _ in range(40):
        new=set(os.listdir(d))-before
        if new: break
        time.sleep(0.2)
    if not new: print(tag,'no png'); continue
    f=os.path.join(d,new.pop())
    for _ in range(30):
        s=os.path.getsize(f); time.sleep(0.3)
        if s>0 and s==os.path.getsize(f): break
    subprocess.run(['cp',f,f'context/b31e/{tag}.png']); print(tag,'vsync',p._u32(VSYNC),'fw',fw,'vis',fl(0x1e7c80),flush=True)
p.write32(0x1bb660,F(0.0)); p.lockstep=False; p.resume()
