"""B507: what the original draws on a human's own lob. Loads <slot> (P1 human), logs the marker object (match
manager +0xa4) each frame like b12_lob_mark.py, and for every shot (the predicted path restarting at 15 entries)
prints the path's first z, the hitter variable and the candidate count, and its peak once the path is 150 long.
After each CPU shot toward P1 it holds △ through tools/vpad.py so P1 returns it as a lob. Run under tools/pcsx2.sh
with tools/vpad.py pressing ✕ now and then (serves).
usage: b507_lob_look.py <slot> <frames> <outdir> [scratch slot: save 30 frames into P1's first rally shot]"""
import sys, time, struct, os, subprocess, json
sys.path.insert(0, "tools")
from pine import Pine
i32 = lambda u: struct.unpack("<i", struct.pack("<I", u))[0]
f32 = lambda b, o: struct.unpack_from("<f", b, o)[0]
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(2)
want, out = int(sys.argv[2]), sys.argv[3]; os.makedirs(out, exist_ok=True)
gm = p.read32(0x422f80); m = p.read32(gm + 0xa4)
hum = [i32(p.read32(m + 400 + 4 * i)) for i in range(4)]
log = open(f"{out}/log.jsonl", "w")
log.write(json.dumps({"humans": hum}) + "\n")
print("humans", hum, flush=True)
prev_len, prev_cnt, last, n, hit = 0, 0, p.read32(0x1d5780), 0, 0
saved, save_at, cpu_before = False, -1, False
while n < want:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    b = p.read_block(m + 0x50, 0x1b0 - 0x50)
    g = lambda o: struct.unpack_from("<I", b, o - 0x50)[0]
    cnt, ln = i32(g(0x144)), i32(g(0x54))
    row = {"v": v, "len": ln, "cnt": cnt, "pend": i32(g(0x184)), "mark": [f32(b, 0x150 - 0x50), f32(b, 0x158 - 0x50)]}
    if ln == 15 and prev_len != 15:
        z0 = struct.unpack_from("<f", p.read_block(g(0x50) + 8, 4))[0]
        hit += 1
        print(f"vsync {v}: hit {hit} from z {z0:.2f} ({'P1 side' if z0 > 0 else 'far side'}?) pend {row['pend']}", flush=True)
        hitter = i32(p.read32(0x423058))
        print(f"  last hitter {hitter}", flush=True)
        if row["pend"]:  # the CPU hit toward P1: hold △ so P1 returns it as a lob
            subprocess.Popen(["tools/vpad.py", "send", "sleep 150", "down triangle", "sleep 900", "up triangle"])
        elif hitter == 0 and cpu_before and len(sys.argv) > 4 and not saved:
            save_at = n + 30  # P1's own rally shot in the air: keep a state to screenshot afterwards
        cpu_before = row["pend"] > 0
    if ln >= 150 and prev_len < 150:
        es = [struct.unpack_from("<4f4fi", p.read_block(g(0x50) + 0x30 * j, 0x24)) for j in range(150)]
        pre = [e for e in es if e[8] == 0]
        print(f"  hit {hit} peak {max(e[1] for e in pre):.2f} m", flush=True)
    if cnt and not prev_cnt:
        print(f"vsync {v}: smash point {row['mark']}", flush=True)
    if n == save_at:
        p.save_state(int(sys.argv[4])); saved = True; print(f"  saved state {sys.argv[4]} at vsync {v}", flush=True)
    prev_len, prev_cnt = ln, cnt
    log.write(json.dumps(row) + "\n")
print("done", n)
