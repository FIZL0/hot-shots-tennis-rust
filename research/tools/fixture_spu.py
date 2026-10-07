# Export a save state's sound state as CSV for hst-data/tests/sound.rs. Usage: fixture_spu.py STATE.p2s OUT.csv
#   bank,<xb on disc>,<entry>,<spu byte address of its .bd>   banks the EE has loaded, found in SPU RAM
#   tone,voice,adsr1,adsr2,spu_addr,pitch_word                every complete key-on (ADSR, sample, pitch) in the EE's
#                                                             SPU command ring (1024 × {cmd, voice, word, word})
#                                                             after the last sample upload (cmd 19), so the
#                                                             loaded banks are the ones that played
#   pitch,voice,pitch_word,scale_word,vp                      each voice's last pitch command and its SPU2 pitch register
# Needs context/xb (xbdump) to name the banks.
import glob, struct, subprocess, sys
st, out = sys.argv[1:3]
z = lambda n: subprocess.run(['7z', 'e', '-so', st, n], capture_output=True, check=True).stdout
ee, spu = z('eeMemory.bin'), z('SPU2.bin')
RING, HEAD = 0x305000, 0x304fc0              # EE command ring and its write index
SPURAM = 0x10004                             # SPU2.bin offset of SPU RAM address 0
VOICE = lambda v: (0x210240 + 0x108 * v) if v < 24 else (0x212200 + 0x108 * (v - 24))  # pitch u16 per voice
disc = {}
for f in sorted(glob.glob('context/xb/SND/**/*.hd', recursive=True)):
    xb, entry = f[len('context/xb/'):].split('.XB', 1)
    xb, entry = xb + '.XB' + entry.split('/', 1)[0], entry.split('/', 1)[1]
    disc.setdefault(len(open(f, 'rb').read()), []).append((f, xb, entry))
rows = []
i = 0
while (i := ee.find(b'SShd', i + 1)) >= 0:
    b = i - 0xc
    hs, bs = struct.unpack_from('<2I', ee, b)
    h = ee[b:b + hs]
    for f, xb, entry in disc.get(hs, []):
        d = open(f, 'rb').read()
        if d[:0x30] != h[:0x30] or d[0x48:] != h[0x48:]:  # 0x30..0x48: pointers the loader fills in
            continue
        bd = open(f[:-2] + 'bd', 'rb').read()
        a = spu.find(bd, SPURAM, SPURAM + 0x200000)
        if a >= 0:
            rows.append(f'bank,{xb},{entry},{a - SPURAM}')
        break
head, = struct.unpack_from('<i', ee, HEAD)
cur, last = {}, {}
ring = [struct.unpack_from('<4I', ee, RING + 16 * (n % 1024)) for n in range(head - 1023, head)]
upload = max(i for i, c in enumerate(ring) if c[0] == 19)
for i, (c, v, w8, wc) in enumerate(ring):
    if i < upload and c != 4: continue
    if c == 2: cur[v] = [w8 & 0xffff, wc & 0xffff]
    elif c == 3 and v in cur and len(cur[v]) == 2: cur[v].append(w8)
    elif c == 4:
        last[v] = (w8, wc)
        if len(cur.get(v, ())) == 3: rows.append('tone,%d,%d,%d,%d,%d' % (v, *cur.pop(v), w8))
for v, (w8, wc) in sorted(last.items()):
    rows.append(f'pitch,{v},{w8},{wc},{struct.unpack_from("<H", spu, VOICE(v))[0]}')
open(out, 'w').write('\n'.join(rows) + '\n')
print(len(rows), 'rows', sum(r.startswith('bank') for r in rows), 'banks')
