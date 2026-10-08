#!/bin/sh
# usage: xr.sh <hexaddr-without-0x> [file]  -> who references it: to, from, function, type (default: the GAME overlay)
grep "^00$1	" "${2:-$(dirname "$(realpath "$0")")/../context/decomp/hst_game.xrefs.tsv}"
