#!/bin/bash
# B211: send vpad commands to PCSX2 copy $HST_PCSX2, then screenshot it at 1024 wide into $1.
out=$1; shift
[ $# = 0 ] || tools/vpad.py send "$@"
sleep 1
tools/screenshot.sh "$out.full.png" >/dev/null
d="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")-slots/pcsx2/s$HST_PCSX2/PCSX2/snaps"
python3 -c "from PIL import Image; import sys; Image.open(sys.argv[1]).convert('RGB').resize((1024,576)).save(sys.argv[2])" "$d/$(ls "$d" | sort -V | tail -1)" "$out"
