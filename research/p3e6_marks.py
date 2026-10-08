"""P3e6: poke test cheer marks into the gallery manager (slot 3, waiting for the serve) and screenshot them.
Writes context/p3e6/{cam.json, k0.png, k1.png, none.png}. Run under tools/pcsx2.sh."""
import json, struct, subprocess, sys, time
sys.path.insert(0, 'tools')
from pine import Pine

MARKS = [(0, -1, 0), (-4, -1, -8), (4, -1, 8), (0, -1, -11.9), (8, -3, 0), (-6, -2, 4)]
p = Pine()
p.load_state(3)
time.sleep(2.0)
f = lambda a: struct.unpack('<f', struct.pack('<I', p.read32(a)))[0]
mgr = p.read32(0x43b1c0)
cam = {'rows': [[f(0x1e7f30 + 16 * r + 4 * j) for j in range(4)] for r in range(4)],
       'view': [[f(0x1e7e30 + 16 * r + 4 * j) for j in range(4)] for r in range(4)],
       'proj': [[f(0x1e7ff0 + 16 * r + 4 * j) for j in range(4)] for r in range(4)],
       'fov': f(0x1e7d50), 'court': p.read32(0x422f90), 'run': p.read8(mgr + 0x1b61)}
extra = [cam['rows'][3][i] + cam['rows'][2][i] * 10 for i in range(3)]
marks = MARKS + [tuple(extra)]
cam['marks'] = marks
fb = lambda v: struct.unpack('<I', struct.pack('<f', v))[0]
def put(kind, n):
    for k, m in enumerate(marks[:n]):
        a = mgr + 0x8d0 + 0x30 * k
        for i, v in enumerate(list(m) + [1.0] + list(m) + [1.0]):
            p.write32(a + 4 * i, fb(v))
        p.write32(a + 0x20, 1000000)
        p.write32(a + 0x24, 0)
    p.write32(mgr + 0x8c4, kind)
    p.write32(mgr + 0x8c8, n)
for kind in (0, 1):
    put(kind, len(marks))
    time.sleep(0.3)
    subprocess.run(['tools/screenshot.sh', f'context/p3e6/k{kind}.png'], check=True)
put(0, 0)
time.sleep(0.3)
subprocess.run(['tools/screenshot.sh', 'context/p3e6/none.png'], check=True)
json.dump(cam, open('context/p3e6/cam.json', 'w'), indent=1)
print(json.dumps(cam))
