"""P11k8: compare the NET routine's stand scratch as recorded on entry (record +0x730, sp-0x20) with what 0x362090
reads (record_ai_rally.py HST_PROBE=1's out.probe; kind-1 rows, the mate record 0x35e560 read after it, are listed).
A vsync can tick between the entry hook and the read, so a probe matches an entry of that AI at v or v-1.
Usage: p11k8_probe.py out.bin"""
import struct, sys
d = open(sys.argv[1], "rb").read()
pr = open(sys.argv[1] + ".probe", "rb").read()
o, ent = 8 + 0x9c8, {}
while o < len(d):
    tag, size = struct.unpack_from("<II", d, o)
    if tag == 2:
        v, a0 = struct.unpack_from("<I", d, o + 8)[0], struct.unpack_from("<I", d, o + 0x10)[0]
        ent.setdefault((v, a0), []).append(struct.unpack_from("<4f", d, o + 0x730))
    o += size
same = diff = 0
for k in range(0, len(pr), 0x20):
    v, a0, x, kind = struct.unpack_from("<4I", pr, k)
    if kind == 1:
        print(f"v{v} mate record {struct.unpack_from('<3i', pr, k + 0x10)}")
        continue
    q = struct.unpack_from("<4f", pr, k + 0x10)
    e = ent.get((v, a0), []) + ent.get((v - 1, a0), [])
    ok = q in e
    same, diff = same + ok, diff + (not ok)
    if not ok:
        print(f"v{v} ai {a0:#x} probe {q} entries {e} DIFF")
print("same", same, "diff", diff)
