"""N4a: the original's yellow smash marker. From slot 4 (P1 human), the opponents' (slots 1, 3) strokes are forced
to △ lobs; the marker object (match manager +0xa4) is logged each frame to <outdir>/log.jsonl, its predicted path
is dumped when the first smash point is placed, PCSX2's F8 screenshots follow, and the first placement is written
as the fixture context/fixtures/smash_mark_s04.txt (hst-sim tests/effect.rs smash_mark_s04).
Run under tools/pcsx2.sh with tools/vpad.py pressing ✕ now and then (P1 serves).
usage: smash_mark_rec.py <slot> <frames> <outdir>"""
import sys, time, struct, os, subprocess, json
sys.path.insert(0, "tools")
from pine import Pine
i32 = lambda u: struct.unpack("<i", struct.pack("<I", u))[0]
f32 = lambda b, o: struct.unpack_from("<f", b, o)[0]
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(2)
want, out = int(sys.argv[2]), sys.argv[3]; os.makedirs(out, exist_ok=True)
pid = open(f"{os.environ['XDG_RUNTIME_DIR']}/hst-pcsx2{os.environ.get('HST_PCSX2','')}.pid").read().strip()
f8 = lambda: subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F8", window = "pid:{pid}" }})'], capture_output=True)
gm = p.read32(0x422f80); m = p.read32(gm + 0xa4)
pl = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
hum = [i32(p.read32(m + 400 + 4 * i)) for i in range(4)]
heights = {i: (lambda b: (f32(b, 0), f32(b, 4)))(p.read_block(0x2f0880 + c * 0x118 + 0xe8, 8)) for i, c in enumerate(hum) if c >= 0}
log = open(f"{out}/log.jsonl", "w")
log.write(json.dumps({"heights": heights, "humans": hum}) + "\n")
last, n, prev_cnt, shots = p.read32(0x1d5780), 0, 0, 0
while n < want:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    for k in (1, 3):
        if i32(p.read32(pl[k] + 0x3ec4)) >= 0 and p.read32(pl[k] + 0x3ee4) != 4:
            p.write32(pl[k] + 0x3ee4, 4)
    b = p.read_block(m + 0x50, 0x1b0 - 0x50)
    g = lambda o: struct.unpack_from("<I", b, o - 0x50)[0]
    cnt = i32(g(0x144))
    row = {"v": v, "len": i32(g(0x54)), "ext": b[0x5c - 0x50], "cnt": cnt, "pend": i32(g(0x184)), "next": i32(g(0x180)),
           "st": [i32(g(0x178)), i32(g(0x17c))], "mark": [f32(b, 0x150 - 0x50), f32(b, 0x158 - 0x50)], "red": b[0x1a0 - 0x50]}
    if cnt and not prev_cnt:
        row["path"] = [list(struct.unpack_from("<4f4fi", p.read_block(g(0x50) + 0x30 * j, 0x24))) for j in range(row["len"])]
        print(f"vsync {v}: smash point {row['mark']} path {row['len']}", flush=True)
        shots = 12
    if shots and n % 6 == 0:
        f8(); shots -= 1
    prev_cnt = cnt
    log.write(json.dumps(row) + "\n")
    if n % 600 == 0: print(n, flush=True)
print("done", n)
log.close()
rows = [json.loads(l) for l in open(f"{out}/log.jsonl")]
first = next((r for r in rows[1:] if "path" in r), None)
if first:
    hx = lambda x: struct.pack(">f", x).hex()
    with open("context/fixtures/smash_mark_s04.txt", "w") as o:
        o.write(" ".join(hx(x) for x in (*heights[0], *first["mark"])) + "\n")
        for e in first["path"]: o.write(" ".join(hx(e[k]) for k in (0, 1, 2, 4, 5, 6)) + f" {e[8]}\n")
