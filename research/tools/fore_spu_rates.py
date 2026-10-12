# Check HST-MODS audio.py's Fore sample rates against the running game: for every sounding SPU2 voice in a PCSX2 save
# state of Hot Shots Golf Fore!, find its sample among the SShd banks (SPU RAM at the voice's start address vs each
# tone's .bd bytes) and print the SPU pitch register as a rate next to the rate audio.py gave that sample.
#   fore_spu_rates.py STATE.p2s [STATE.p2s ...]   (needs ../HST-MODS/out/files/fore from its build.sh)
import os, struct, subprocess, sys
MODS = os.path.join(os.path.dirname(subprocess.run(['git', '-C', os.path.dirname(os.path.abspath(__file__)), 'rev-parse',
    '--path-format=absolute', '--git-common-dir'], capture_output=True, text=True).stdout.strip()), '../HST-MODS')
sys.path.insert(0, f'{MODS}/tools')
states, sys.argv = sys.argv[1:], sys.argv[:1] + ['fore']  # audio.py reads its own argv on import
from audio import fore_tones, fore_rate
SPURAM, KEY = 0x10004, 64
VOICE = lambda v: (0x210240 + 0x108 * v) if v < 24 else (0x212200 + 0x108 * (v - 24))
idx = {}  # first KEY bytes of a sample -> {(bank, semitones)}
for d, _, fs in os.walk(f'{MODS}/out/files/fore'):
    for n in fs:
        if not n.lower().endswith('.hd') or not os.path.exists(bd := os.path.join(d, n[:-2] + n[-2:].translate(str.maketrans('hH', 'bB')))):
            continue
        try:
            tones = fore_tones(open(os.path.join(d, n), 'rb').read())
        except Exception:
            continue
        b = open(bd, 'rb').read()
        for a, (s, note, st) in tones.items():
            k = b[a:a + KEY]
            if len(k) == KEY and any(k[16:]):
                idx.setdefault(k, set()).add((os.path.relpath(os.path.join(d, n), f'{MODS}/out/files/fore'), st))
for st_path in states:
    spu = subprocess.run(['7z', 'e', '-so', st_path, 'SPU2.bin'], capture_output=True, check=True).stdout
    print('#', st_path)
    for v in range(48):
        b = VOICE(v) - 80
        phase, = struct.unpack_from('<B', spu, b + 76)
        pitch, lsa, ssa = struct.unpack_from('<HxxII', spu, b + 80)
        if not phase:
            continue
        hits = idx.get(spu[SPURAM + 2 * ssa:SPURAM + 2 * ssa + KEY], set())
        sts = {st for _, st in hits}
        game = round(pitch * 48000 / 4096)
        if len(sts) == 1:
            st = sts.pop()
            bank = sorted(h for h, _ in hits)[0]
            print(f'v{v:2d} pitch={pitch:5d} game={game:5d} Hz  audio.py={fore_rate(st):5d} Hz ({st:+.4f} st) '
                  f'{"OK" if abs(game - fore_rate(st)) <= 0.002 * game else "DIFF"}  {bank} (+{len(hits) - 1})')
        else:
            print(f'v{v:2d} pitch={pitch:5d} game={game:5d} Hz  sample {"not found" if not hits else f"ambiguous {sorted(sts)}"}')
