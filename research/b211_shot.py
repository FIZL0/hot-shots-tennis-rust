"""B211: drive slot 2's menu to a doubles match (court varies on the confirm screen; court 5 wanted), wait for the
first serve set-up, screenshot the original OUTDIR/orig.png and write OUTDIR/eye (HST_EYE) for the same view in
`--shot`. usage: b211_shot.py OUTDIR [delay_frames [slot]]: slot 8 = a confirm screen with Queen's picked (saved by hand from slot 2:
✕ ×4, Start, Select Court, thumbnail 11). Run under tools/pcsx2.sh."""
import math, os, struct, subprocess, sys, time
sys.path.insert(0, 'tools')
from pine import Pine

out = sys.argv[1]
delay = int(sys.argv[2]) if len(sys.argv) > 2 else 60
os.makedirs(out, exist_ok=True)
p = Pine()
slot = int(sys.argv[3]) if len(sys.argv) > 3 else 8
p.load_state(slot)
time.sleep(1.5)
route = ['press up 120', 'sleep 700', 'press cross 120']
subprocess.run(['tools/vpad.py', 'send', *route], check=True)
f = lambda a: struct.unpack('<f', struct.pack('<I', p.read32(a)))[0]
t0 = time.time()
while time.time() - t0 < 180:
    gm = p.read32(0x422f80)
    if gm and p.read32(0x422f90) not in (0, 5):
        sys.exit(f'court {p.read32(0x422f90)}, not 5: run it again')
    if gm and p.read8(gm + 0x55) == 2:
        break
    time.sleep(0.05)
else:
    sys.exit('CAPTURE FAILED: no serve set-up in 180 s')
time.sleep(delay / 60)
shot = subprocess.Popen(['tools/screenshot.sh', f'{out}/orig.png'])
rows = [[f(0x1e7f30 + 16 * r + 4 * j) for j in range(4)] for r in range(4)]
t = math.tan(math.radians(f(0x1e7d50)) / 2) * 4 / 3
open(f'{out}/eye', 'w').write(','.join(f'{v:.6f}' for v in rows[3][:3] + rows[2][:3] + rows[1][:3] + [t]))
assert shot.wait() == 0, 'CAPTURE FAILED: no screenshot'
time.sleep(3)
print('court', p.read32(0x422f90), 'eye', open(f'{out}/eye').read())
