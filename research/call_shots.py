"""P12b3: the original's call pop-ups (Let/Out/Net/Fault/Double Fault, Change Sides). Loads a save slot, polls the
scoreboard every vsync while a call shows (kind 5/7) and logs its state (model, fade stage/countdown, settled, anim
time, voice countdown), F8 every 2nd frame of the first `calls` calls; dumps the overlay camera (+0x680 of the
scoreboard's owner) and the model's world matrix once.
usage: call_shots.py <slot> <max_frames> <outdir> [calls]"""
import sys, time, os, subprocess, struct
sys.path.insert(0, "tools")
from pine import Pine
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
want, out = int(sys.argv[2]), sys.argv[3]; os.makedirs(out, exist_ok=True)
calls = int(sys.argv[4]) if len(sys.argv) > 4 else 2
pid = open(f"{os.environ['XDG_RUNTIME_DIR']}/hst-pcsx2{os.environ.get('HST_PCSX2','')}.pid").read().strip()
f8 = lambda: subprocess.run(["hyprctl", "dispatch", f'hl.dsp.send_shortcut({{ mods = "", key = "F8", window = "pid:{pid}" }})'], capture_output=True)
log = open(f"{out}/calls.txt", "w")
sb = p.read32(0x42d6c0)
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
last, n, seen, prev, dumped = p.read32(0x1d5780), 0, 0, 0, False
while n < want and seen <= calls:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    kind = p.read8(sb + 0x148)
    if kind in (5, 7) and not p.read8(sb + 0x151):
        if prev not in (5, 7):
            seen += 1; k = 0
        w = p.read32(sb + 0x58); pv = p.read32(w + 4); mdl = p.read32(pv)
        time_ = f(pv + 0x38); ani = p.read32(pv + 0x24); end = f(ani + 0x2c) if ani else 0.0
        alpha = f(p.read32(mdl + 0xc) + 0x5c)
        line = (f"{n} {v} call#{seen} k={k} kind={kind} model={p.read8(sb+0x427)} call={p.read8(sb+0x426)} fade={p.read8(sb+0x150)} "
                f"t={p.read32(sb+0x154)} settled={p.read8(sb+0x158)} held={p.read32(sb+0x14c)} cd={p.read32(0x42d6e8)} "
                f"anim={time_} end={end} alpha={alpha}")
        if seen <= calls and k % 2 == 0:
            f8(); line += " SHOT"
        log.write(line + "\n"); log.flush()
        if not dumped:
            gm = p.read32(0x43b1d8)
            open(f"{out}/camera.bin", "wb").write(p.read_block(gm + 0x680, 0x580))
            open(f"{out}/model.bin", "wb").write(p.read_block(mdl, 0x140))
            open(f"{out}/wrapper.bin", "wb").write(p.read_block(w, 0x60))
            dumped = True
        k += 1
    prev = kind
print("done", n, seen)
