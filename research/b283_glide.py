"""B283: the original's camera during the change-of-ends glide on court 5: from scratch slot 8 (see b283_ends.py) P1
presses ○ until the eye leaves its end (|z| < 37), then screenshots + HST_EYE (OUTDIR/gN.png, gN.eye) whenever the
eye has moved ≥ 6 m since the last one, until it reaches the other end. usage: b283_glide.py OUTDIR. Run under
tools/pcsx2.sh."""
import math, os, struct, subprocess, sys, time
sys.path.insert(0, 'tools')
from pine import Pine

out = sys.argv[1]
os.makedirs(out, exist_ok=True)
p = Pine()
p.load_state(8)
time.sleep(3)
subprocess.run(['tools/vpad.py', 'send', 'release', 'sleep 500', 'press cross 150', 'sleep 1200', 'stick l 0 -1', 'sleep 200', 'stick l 0 0', 'sleep 800', 'press cross 150'], check=True)
f = lambda a: struct.unpack('<f', struct.pack('<I', p.read32(a)))[0]
eye = lambda: [f(0x1e7f30 + 0x30 + 4 * i) for i in range(3)]
t0 = time.time()
while not (p.read32(0x422f80) and abs(eye()[2]) > 37):
    if time.time() - t0 > 180:
        sys.exit('CAPTURE FAILED: no match camera')
    time.sleep(0.2)
if p.read32(0x422f90) != 5:
    sys.exit(f'court {p.read32(0x422f90)}, not 5')
z0 = eye()[2]
while abs(eye()[2]) > 37 or eye()[2] * z0 < 0:
    if time.time() - t0 > 900:
        sys.exit('CAPTURE FAILED: no change of ends')
    subprocess.run(['tools/vpad.py', 'send', 'press circle 100'], check=True)
    time.sleep(0.3)
last, n = None, 0
while n < 8:
    e = eye()
    if last is None or math.dist(e, last) >= 6:
        rows = [[f(0x1e7f30 + 16 * r + 4 * j) for j in range(4)] for r in range(4)]
        t = math.tan(math.radians(f(0x1e7d50)) / 2)
        open(f'{out}/g{n}.eye', 'w').write(','.join(f'{v:.6f}' for v in rows[3][:3] + rows[2][:3] + rows[1][:3] + [t]))
        # PINE reads lead the screenshot (taken from the snap folder a moment later): two, keep the second
        subprocess.run(['tools/screenshot.sh', f'{out}/g{n}.png'], check=True)
        subprocess.run(['tools/screenshot.sh', f'{out}/g{n}.png'], check=True)
        print(n, open(f'{out}/g{n}.eye').read(), flush=True)
        last, n = e, n + 1
    if abs(e[2]) > 37 and e[2] * z0 < 0:
        break
    time.sleep(0.05)
