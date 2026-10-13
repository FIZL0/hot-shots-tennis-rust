"""B138: wait for the walkers' cheer marks in the original and screenshot them.
usage: b138_claps.py SLOT OUTDIR [delay_frames [hold_s]] — load SLOT, poll until the gallery manager shows walker marks
(kind 0, count > 0), wait delay_frames vsyncs, screenshot (live: it can't while paused) OUTDIR/orig.png and write OUTDIR/eye (HST_EYE) and
OUTDIR/marks.json (the marks' pos/base). Run under tools/pcsx2.sh."""
import json, math, struct, subprocess, sys, time
sys.path.insert(0, 'tools')
from pine import Pine

slot, out = int(sys.argv[1]), sys.argv[2]
delay = int(sys.argv[3]) if len(sys.argv) > 3 else 20
hold = float(sys.argv[4]) if len(sys.argv) > 4 else 0
p = Pine()
p.load_state(slot)
time.sleep(2.0)
f = lambda a: struct.unpack('<f', struct.pack('<I', p.read32(a)))[0]
mgr = p.read32(0x43b1c0) & 0x1ffffff
t0 = time.time()
while time.time() - t0 < 600:
    if p.read32(mgr + 0x8c8) > 0 and p.read32(mgr + 0x8c4) == 0:
        break
    time.sleep(0.01)
else:
    sys.exit('CAPTURE FAILED: no walker marks in 600 s')
time.sleep(delay / 60)
# the hotkey grabs the frame it lands on: press it first, read the marks meanwhile
shot = subprocess.Popen(['tools/screenshot.sh', f'{out}/orig.png'])
if hold:  # keep the marks shown past the next point's reset (the hotkey's frame lands a second or two late)
    n0, t1 = p.read32(mgr + 0x8c8), time.time()
    while time.time() - t1 < hold:
        if p.read32(mgr + 0x8c8) != n0:
            p.write32(mgr + 0x8c8, n0)
rows = [[f(0x1e7f30 + 16 * r + 4 * j) for j in range(4)] for r in range(4)]
fov = f(0x1e7d50)
n = p.read32(mgr + 0x8c8)
marks = [[f(mgr + 0x8d0 + 0x30 * k + 4 * i) for i in range(8)] + [p.read32(mgr + 0x8f0 + 0x30 * k)] for k in range(n)]
t = math.tan(math.radians(fov) / 2) * 4 / 3  # fov = the full vertical angle in degrees → horizontal half tan
eye = rows[3][:3] + rows[2][:3] + rows[1][:3] + [t]
open(f'{out}/eye', 'w').write(','.join(f'{v:.6f}' for v in eye))
json.dump({'rows': rows, 'fov': fov, 'court': p.read32(0x422f90), 'marks': marks}, open(f'{out}/marks.json', 'w'), indent=1)
assert shot.wait() == 0, 'CAPTURE FAILED: no screenshot'
time.sleep(3)  # the PNG lands after screenshot.sh returns
print(n, 'marks; fov', fov, 'court', p.read32(0x422f90))
