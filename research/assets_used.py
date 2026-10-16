#!/usr/bin/env python3
"""Z2: fold asset-use logs (`HST_ASSET_LOG=FILE hst …`, hst_data::uselog) into what was loaded but never used.

usage: research/assets_used.py TEXKEYS LOG... [--pack DIR]

TEXKEYS is `target/release/texkeys <iso>` output (every disc texture's key and `--dump-textures` name). Prints, per
disc archive: entries decoded; texture entries (MTL/tm2) none of whose textures was ever drawn; models whose sibling
MTL was never drawn; sound banks never played; and with --pack, a texture pack's files whose key was never even looked
up (`tex-key`) or looked up but never drawn. "Never" means in these logs: only as good as the runs behind them.
"""
import collections
import os
import re
import sys


def norm(archive, entry):
    """A log `xb` line's archive + entry as a --dump-textures name without `.N` (lower case)."""
    base = re.sub(r"\.XB0?$", "", archive)
    stem = entry.rsplit(".", 1)[0]
    parts = [p for p in re.split(r"[\\/]", stem) if p not in ("", ".", "..")]
    return (base + "/" + "/".join(parts)).lower()


def main():
    args = sys.argv[1:]
    pack = None
    if "--pack" in args:
        i = args.index("--pack")
        pack = args[i + 1]
        del args[i:i + 2]
    if len(args) < 2:
        sys.exit(__doc__)
    names_of = collections.defaultdict(set)  # dump name stem -> keys
    for line in open(args[0]):
        k, p = line.rstrip("\n").split("\t")
        names_of[p.rsplit(".", 1)[0].lower()].add(k)
    seen = collections.defaultdict(set)
    for log in args[1:]:
        for line in open(log, errors="replace"):
            kind, _, name = line.rstrip("\n").partition("\t")
            seen[kind].add(name)
    drawn, looked = seen["tex-draw"], seen["tex-key"]
    played = {n.split(" ")[0] for n in seen["snd"]}
    imgs = seen["img-draw"]
    entries = collections.defaultdict(list)
    for n in seen["xb"]:
        a, _, e = n.partition(":")
        entries[a].append(e)
    unused = collections.defaultdict(list)
    for a, es in sorted(entries.items()):
        for e in sorted(es):
            stem, ext = (e.rsplit(".", 1) + [""])[:2]
            ext = ext.lower()
            keys = names_of.get(norm(a, e), set())
            if f"{a}:{e}" in imgs:
                continue
            if ext in ("mtl", "tm2", "bmp") and keys and not keys & drawn:
                if keys & looked:
                    unused[a].append(f"{e}  (texture made, never drawn)")
                else:
                    unused[a].append(f"{e}  (decoded, no image made by textures::add: check by hand)")
            elif ext == "mdl":
                mk = names_of.get(norm(a, stem + ".MTL"), set())
                if mk and not mk & drawn:
                    unused[a].append(f"{e}  (its MTL never drawn)")
            elif ext == "hd" and f"{a}:{e}" not in played:
                unused[a].append(f"{e}  (bank never played)")
    total = sum(len(v) for v in entries.values())
    print(f"{len(seen['disc'])} disc files, {total} entries decoded, {len(looked)} texture keys made, "
          f"{len(drawn)} drawn, {len(played)} sound banks played")
    for a, us in unused.items():
        print(f"\n{a}: {len(us)} of {len(entries[a])} entries unused")
        for u in us:
            print("   ", u)
    if pack:
        never, undrawn, n = [], [], 0
        for root, _, files in os.walk(pack):
            for f in files:
                m = re.search(r"-([0-9a-f]{16})\.png$", f, re.I)
                if not m:
                    continue
                n += 1
                k, path = m.group(1), os.path.join(root, f)
                if k not in looked:
                    never.append(path)
                elif k not in drawn:
                    undrawn.append(path)
        print(f"\npack {pack}: {n} files, {len(never)} never looked up, {len(undrawn)} looked up but never drawn")
        out = os.environ.get("Z2_PACK_LIST")
        if out:
            with open(out, "w") as f:
                f.writelines(p + "\n" for p in sorted(never))


if __name__ == "__main__":
    main()
