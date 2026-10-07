#!/bin/sh
# usage: fn.sh <hexaddr-without-0x> [file]  -> print decompiled function (default: the GAME overlay's dump)
awk -v a="// 00$1 " 'index($0,a)==1{p=1;print;next} p&&/^\/\/ 00/{exit} p' "${2:-$(dirname "$(realpath "$0")")/../context/decomp/hst_game.c}"
