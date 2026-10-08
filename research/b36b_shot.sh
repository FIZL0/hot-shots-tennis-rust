#!/bin/bash
# B36b: screenshot slot 5 (court 10, bot match) n frames after loading it. Usage: b36b_shot.sh n out.png
set -e
cd "$(dirname "$0")/.."
python3 - "$1" <<'P'
import sys; sys.path.insert(0, 'tools'); from pine import Pine
p = Pine(); p.load_state(5); v0 = v = p.read32(0x1d5780)
while v - v0 < int(sys.argv[1]): v = p.next_frame(v)
P
tools/screenshot.sh "$2"
