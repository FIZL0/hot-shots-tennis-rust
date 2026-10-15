#!/usr/bin/env python3
"""Pull iPhoneOS.sdk out of an Xcode .xip on Linux (no Mac, no sudo).

Usage: tools/ios-sdk.py ~/Downloads/Xcode_X.xip [OUT_DIR]   (default ~/.local/share/hst-ios → OUT_DIR/iPhoneOS.sdk)

A .xip is a xar (bsdtar reads it) holding a pbzx stream (xz chunks) of an odc cpio. Hard-linked files keep their
bytes in one entry only; the other links hold a "NULLcanary" placeholder of the same size, so GNU cpio extracts
garbage for them. Pass 1 notes which hard-linked inodes the SDK needs, pass 2 takes their bytes from whichever
entry has them. The odc inode field is 18 bits and wraps, so links are matched on device + inode + size.

Linker stubs (.tbd): Xcode 27's list only arm64e and the new arm64e.x1, which no lld knows. An arm64 app links
against arm64e system libraries (Apple's ld allows it; the phone's libraries are arm64e), so arm64e.x1 is renamed
arm64 in every target list that doesn't already have arm64.
"""
import lzma, os, re, stat, struct, subprocess, sys

TBD_LIST = re.compile(r"\[([^\]]*arm64e\.x1-ios[^\]]*)\]")
SDK = re.compile(r"/SDKs/iPhoneOS[\d.]*\.sdk/")
CANARY = b"NULLcanary"


def entries(xip):
    p = subprocess.Popen(["bsdtar", "-xOf", xip, "Content"], stdout=subprocess.PIPE)
    r, buf = p.stdout, bytearray()

    def chunks():
        assert r.read(4) == b"pbzx", "not a pbzx stream"
        r.read(8)  # flags
        while h := r.read(16):
            size, csize = struct.unpack(">QQ", h)
            c = r.read(csize)
            yield c if csize == size else lzma.decompress(c)

    g = chunks()

    def read(n):
        while len(buf) < n:
            c = next(g, None)
            if c is None:
                break
            buf.extend(c)
        out = bytes(buf[:n])
        del buf[:n]
        return out

    while len(h := read(76)) == 76:
        assert h[:6] == b"070707", f"not an odc cpio entry: {h[:6]!r}"
        mode, nlink, size = int(h[18:24], 8), int(h[36:42], 8), int(h[65:76], 8)
        ino = (h[6:18], size)  # dev + 18-bit inode wraps on Xcode's file count; the size tells collisions apart
        name = read(int(h[59:65], 8))[:-1].decode()
        data = read(size)
        if name == "TRAILER!!!":
            break
        yield ino, mode, nlink, name, data
    p.stdout.close()
    p.wait()


def sdk_path(name):
    m = SDK.search(name)
    return name[m.end():] if m else None


def main():
    xip = sys.argv[1]
    out = os.path.join(sys.argv[2] if len(sys.argv) > 2 else os.path.expanduser("~/.local/share/hst-ios"), "iPhoneOS.sdk")
    if os.path.exists(out):
        sys.exit(f"{out} exists: remove it first")
    need = {ino for ino, mode, nlink, name, _ in entries(xip) if nlink > 1 and stat.S_ISREG(mode) and sdk_path(name)}
    have, pending, files = {}, {}, 0  # inode → bytes; inode → [(path, mode)] still waiting for bytes
    for ino, mode, nlink, name, data in entries(xip):
        if ino in need and not data.startswith(CANARY):
            if have.setdefault(ino, data) != data:
                sys.exit(f"two different files share link key {ino}: {name}")
            for path, m in pending.pop(ino, []):
                write(path, m, data)
        rel = sdk_path(name)
        if rel is None:
            continue
        path = os.path.join(out, rel)
        if stat.S_ISDIR(mode):
            os.makedirs(path, exist_ok=True)
        elif stat.S_ISLNK(mode):
            os.makedirs(os.path.dirname(path), exist_ok=True)
            os.symlink(data.decode(), path)
        elif ino in need and data.startswith(CANARY):
            if ino in have:
                write(path, mode, have[ino])
            else:
                pending.setdefault(ino, []).append((path, mode))
            files += 1
        else:
            write(path, mode, data)
            files += 1
    if pending:
        sys.exit(f"{sum(map(len, pending.values()))} hard-linked files never got their bytes")
    if not files:
        sys.exit(f"no iPhoneOS SDK in {xip}")
    for root, dirs, _ in os.walk(out):  # directories were made writable for extraction
        for d in dirs:
            os.chmod(os.path.join(root, d), 0o755)
    print(out, f"({files} files)")


def arm64_tbd(text):
    def fix(m):
        ts = [t.strip() for t in m.group(1).split(",")]
        ts = [t for t in ts if t != "arm64e.x1-ios"] if "arm64-ios" in ts else ["arm64-ios" if t == "arm64e.x1-ios" else t for t in ts]
        return "[ " + ", ".join(ts) + " ]"
    return TBD_LIST.sub(fix, text)


def write(path, mode, data):
    if path.endswith(".tbd"):
        data = arm64_tbd(data.decode()).encode()
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)
    os.chmod(path, stat.S_IMODE(mode) | 0o200)


assert arm64_tbd("targets: [ arm64e-ios, arm64e.x1-ios ]") == "targets: [ arm64e-ios, arm64-ios ]"
assert arm64_tbd("[ arm64-ios, arm64e-ios, arm64e.x1-ios ]") == "[ arm64-ios, arm64e-ios ]"

if __name__ == "__main__":
    main()
