"""B283: court 5 (grass) from both ends in the original. Starts a doubles match on it from scratch slot 8 (saved
by hand: slot 2, ✕ ×4, Start, Select Court, tile 11), screenshots the first serve set-up (OUTDIR/near.png + near.eye,
HST_EYE for `--shot`), then P1 presses ○ every ~1.5 s until the camera's eye is at the other end of the court and
does the same again after the next serve set-up (far.png, far.eye). usage: b283_ends.py OUTDIR. Run under
tools/pcsx2.sh."""
import math, os, struct, subprocess, sys, time
sys.path.insert(0, 'tools')
from pine import Pine

out = sys.argv[1]
os.makedirs(out, exist_ok=True)
p = Pine()
p.load_state(8)
time.sleep(3)
# slot 8: the confirm screen's court picker on Queen's (slot 2: ✕ ×4, Start, Select Court, tile 11)
subprocess.run(['tools/vpad.py', 'send', 'release', 'sleep 500', 'press cross 150', 'sleep 1200', 'stick l 0 -1', 'sleep 200', 'stick l 0 0', 'sleep 800', 'press cross 150'], check=True)
f = lambda a: struct.unpack('<f', struct.pack('<I', p.read32(a)))[0]
phase = lambda: p.read8(p.read32(0x422f80) + 0x55) if p.read32(0x422f80) else -1
eye_z = lambda: f(0x1e7f30 + 0x38)

def setup(t_max):
    t0 = time.time()
    while time.time() - t0 < t_max:
        gm = p.read32(0x422f80)
        if gm and p.read32(0x422f90) not in (0, 5):
            sys.exit(f'court {p.read32(0x422f90)}, not 5: run it again')
        if phase() == 2:
            return
        time.sleep(0.05)
    sys.exit('CAPTURE FAILED: no serve set-up')

def shoot(name):
    time.sleep(1.0)
    # (a parallel copy's screenshot can come back as the snap before: the second one is this moment's)
    subprocess.run(['tools/screenshot.sh', f'{out}/{name}.png'], check=True)
    shot = subprocess.Popen(['tools/screenshot.sh', f'{out}/{name}.png'])
    rows = [[f(0x1e7f30 + 16 * r + 4 * j) for j in range(4)] for r in range(4)]
    t = math.tan(math.radians(f(0x1e7d50)) / 2)  # (B211's × 4 / 3 frames wider than the original)
    open(f'{out}/{name}.eye', 'w').write(','.join(f'{v:.6f}' for v in rows[3][:3] + rows[2][:3] + rows[1][:3] + [t]))
    assert shot.wait() == 0, 'CAPTURE FAILED: no screenshot'
    print(name, 'court', p.read32(0x422f90), 'eye', open(f'{out}/{name}.eye').read(), flush=True)

setup(180)
shoot('near')
z0, t0 = eye_z(), time.time()
while time.time() - t0 < 600:
    subprocess.run(['tools/vpad.py', 'send', 'press circle 100'], check=True)
    time.sleep(1.4)
    if phase() == 2 and eye_z() * z0 < 0:
        break
else:
    sys.exit('CAPTURE FAILED: no change of ends in 600 s')
shoot('far')
