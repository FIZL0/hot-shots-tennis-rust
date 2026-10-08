# B40b3: on the Data → Items → Characters screen (scratch slot 9), press R2/L2/△ on a character and log, per frame,
# the inspect object's +0x840 flag, motion time, root world matrix translation and Bip01Pelvis's world position.
# usage: tools/pcsx2.sh python3 research/b40b3_probe.py <grid moves right> <button> [frames] > out.tsv
import struct, sys, time
sys.path.insert(0, "tools")
import pine
from vpad import send

OBJ = 0x93d1c0  # the inspect object in this state (axis 520,176 at +0x700)
p = pine.Pine()
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
p.load_state(9)
time.sleep(1.5)
for _ in range(int(sys.argv[1])):
    send(["press right 80"]); time.sleep(0.6)
time.sleep(2.0)
sub = OBJ + 0x80
s = p.read32(p.read32(p.read32(sub + 8)))
skel = p.read32(s + 0xc)
joints, count = p.read32(skel + 100), p.read32(skel + 0x60)
def name(j):
    a = p.read32(p.read32(j + 0x108) + 8); b = p.read_block(a, 32)
    return b.split(b"\0")[0].decode(errors="replace")
pel = next(joints + k * 0x120 for k in range(count) if name(joints + k * 0x120) == "Bip01Pelvis")
mp = p.read32(OBJ + 0x88)  # motion player: +0x3c time
print(f"# char {p.read8(OBJ + 0x7cf)} hand {p.read8(OBJ + 0x7f0)} pelvis {pel:#x} root {s:#x}", flush=True)
buttons = sys.argv[2].split(",")
frames = int(sys.argv[3]) if len(sys.argv) > 3 else 150
v = p.read32(pine.VSYNC)
print("btn\tframe\t840\ttime\tyaw\troot_x\troot_y\troot_z\tpel_x\tpel_y\tpel_z")
for btn in buttons:
    send([f"press {btn} 80"])
    for k in range(frames):
        v = p.next_frame(v)
        row = [btn, k, p.read32(OBJ + 0x840), f(mp + 0x3c), f(OBJ + 0x754)] + [f(s + 0x110 + 4 * i) for i in range(3)] + [f(pel + 0x30 + 4 * i) for i in range(3)]
        print("\t".join(str(x) for x in row), flush=True)
