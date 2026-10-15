#!/usr/bin/env bash
# No retail code addresses or Ghidra names in shipped code (AGENT.md): greps crates/*/src for FUN_/DAT_ names and
# for every function entry address in context/decomp/*.funcs.tsv (skipped when context/ isn't there). Run by done.sh.
# exe.rs (the documented data-offset exception) and hst-oracle (looks functions up by address) are exempt.
cd "$(dirname "$(realpath "$0")")/.."
files=(':(glob)crates/*/src/**' ':!crates/hst-data/src/exe.rs' ':!crates/hst-oracle')
hits=$(git grep -nE '\b(FUN|DAT|LAB|PTR)_[0-9a-fA-F]{6,}' -- "${files[@]}")
if compgen -G 'context/decomp/*.funcs.tsv' >/dev/null; then
  # every 0x… literal, kept when it's a function entry (a 9.6k-pattern grep -f takes minutes)
  hits+=$'\n'$(git grep -noiE '\b0x[0-9a-f]{5,8}\b' -- "${files[@]}" |
    awk -F: 'NR==FNR { a[tolower(gensub(/^0+/, "", 1, $1))]; next }
             { h = tolower($NF); sub(/^0x0*/, "", h) } h in a' <(cut -f1 context/decomp/*.funcs.tsv) -)
fi
hits=$(grep . <<<"$hits") || exit 0
echo "AUDIT FAILED: retail addresses / Ghidra names in shipped code (describe the behaviour instead):"
echo "$hits" | head -20
exit 1
