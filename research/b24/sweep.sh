#!/usr/bin/env bash
# B24: record the same scene at several PCSX2 speeds (NominalScalar; 0 = unlimited); run under tools/pcsx2.sh.
#   research/b24/sweep.sh <name> "<recorder cmd, @ = output file>" <scalar...>
#   e.g. research/b24/sweep.sh live "tools/record_live.py 5 600 @" 0.5 4 0   → context/b24/live_<scalar>.bin + .log
# then research/b24/cmp.py <sample size> context/b24/live_0.5.bin context/b24/live_4.bin
set -e
cd "$(dirname "$0")/../.."
INI=$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")-slots/pcsx2/s${HST_PCSX2:?}/PCSX2/inis/PCSX2.ini
mkdir -p context/b24
name=$1 cmd=$2; shift 2
for s in "$@"; do
    tools/pcsx2-hst.sh stop; sleep 2
    sed -i "s/^NominalScalar = .*/NominalScalar = $s/" "$INI"
    setsid tools/pcsx2-hst.sh >/dev/null 2>&1 & sleep 12
    out=context/b24/${name}_$s
    t=$(date +%s%N)
    timeout 300 python3 ${cmd//@/$out.bin} >$out.log 2>&1 || echo "exit $?" >>$out.log
    echo "wall $(( ($(date +%s%N) - t) / 1000000 )) ms" >>$out.log
    echo "== $s: $(grep -c missed $out.log) miss lines, $(tail -2 $out.log | tr '\n' ' ')"
done
tools/pcsx2-hst.sh stop
sed -i "s/^NominalScalar = .*/NominalScalar = 2/" "$INI"  # a copy's default speed (pcsx2-hst.sh)
