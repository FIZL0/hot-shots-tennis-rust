"""Read tools/record_effects.py captures: yields per frame a dict of the regions (see that script's docstring)."""
import struct, sys

def frames(path):
    d = open(path, "rb").read()
    n = struct.unpack_from("<I", d, 0)[0]
    counts = struct.unpack_from(f"<{2 * n}I", d, 4)
    p = 4 + 8 * n
    sizes = [("fx", 0x100), ("imp", 0x60), ("ball", 0x290), ("mark", 0x40)]
    for k in range(n):
        nm, nt = counts[2 * k], counts[2 * k + 1]
        sizes += [(f"m{k}", 0x80), (f"mesh{k}", 0x60), (f"mor{k}", 0x30), (f"mta{k}", 0x30)]
        sizes += [(f"me{k}_{i}", 0x28) for i in range(nm)] + [(f"te{k}_{i}", 0x20) for i in range(nt)] + [(f"tc{k}_{i}", 0x10) for i in range(nt)]
    size = 4 + sum(s for _, s in sizes)
    while p + size <= len(d):
        f, q = {"vsync": struct.unpack_from("<I", d, p)[0], "counts": counts}, p + 4
        for name, s in sizes:
            f[name] = d[q:q + s]; q += s
        yield f
        p += size

def F(b, o, n=1): return struct.unpack_from(f"<{n}f", b, o) if n > 1 else struct.unpack_from("<f", b, o)[0]
def U(b, o): return struct.unpack_from("<I", b, o)[0]

if __name__ == "__main__":
    prev = None
    for i, f in enumerate(frames(sys.argv[1])):
        imp = f["imp"]; st = U(imp, 0x50)
        if st == 1 or prev == 1:
            fx = f["fx"]; k = [U(fx, 0x84 + 4 * j) for j in range(6)].index(U(imp, 0x54)) if U(imp, 0x54) else -1
            nm, nt = f["counts"][2 * k], f["counts"][2 * k + 1]
            mor, mta, m = f[f"mor{k}"], f[f"mta{k}"], f[f"m{k}"]
            print(i, f["vsync"], "st", st, "kind", k, "d0", U(fx, 0xd0), "mdl t", F(m, 0x38, 3), "mor len/app/t", F(mor, 0x14), F(mor, 0x1c), F(mor, 0x20),
                  "mta", F(mta, 0x14), F(mta, 0x1c), F(mta, 0x20), "w", [round(F(f[f'me{k}_{j}'], 0x24), 4) for j in range(nm)][:4],
                  "a", [round(F(f[f'te{k}_{j}'], 0xc), 4) for j in range(nt)], "scale", F(f[f"mesh{k}"], 0x40))
        prev = st
