#!/usr/bin/env python3
"""B77: peak level of a PCSX2 audio capture (pause_sound.py's companion) per 0.1 s. usage: <wav>"""
import sys, wave, array
w = wave.open(sys.argv[1]); a = array.array('h', w.readframes(w.getnframes())); r = w.getframerate()
for s in range(0, len(a) // 2, r // 10):
    seg = a[2 * s:2 * (s + r // 10)]
    print(f"{s / r:5.1f} {max(map(abs, seg)):6d}", end='  |  ' if (s // (r // 10)) % 5 != 4 else '\n')
