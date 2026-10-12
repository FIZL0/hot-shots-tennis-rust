"""B93: screenshots of the original's held "Game/Set" + "Server/Receiver" banner in every case the draw handles.
Plays slot 5 to its first held banner (stage +0x578 = 2) and saves scratch slot 8 there, then for each case loads
slot 8, pokes the words the draw reads each frame (server 0x42304c, winner team 0x4230a8, result kind 0x4230b8)
and screenshots. usage: b93_banner_cases.py <outdir>"""
import sys, time, os, subprocess
sys.path.insert(0, "tools")
from pine import Pine
out = sys.argv[1]; os.makedirs(out, exist_ok=True)
p = Pine(); p.load_state(5); time.sleep(1)
sb = p.read32(0x42d6c0)
while p.read8(sb + 0x578) != 2 or not p.read8(sb + 0x571): pass
p.save_state(8); time.sleep(2)
# (name, server, winner, kind): server team = server is 1 or 3
for name, server, winner, kind in [("recv_blue_game", 0, 1, 1), ("serv_blue_game", 1, 1, 1), ("serv_red_game", 0, 0, 1),
                                   ("recv_red_game", 1, 0, 1), ("serv_red_set", 2, 0, 2), ("recv_blue_set", 2, 1, 2)]:
    p.load_state(8); time.sleep(0.5)
    p.write32(0x42304c, server); p.write32(0x4230a8, winner); p.write32(0x4230b8, kind)
    time.sleep(0.2)
    subprocess.run(["tools/screenshot.sh", f"{out}/{name}.png"], check=True)
    print(name, "on", p.read8(sb + 0x571), "stage", p.read8(sb + 0x578), flush=True)
