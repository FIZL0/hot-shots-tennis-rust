"""Minimal glTF 2.0 binary (.glb) writer."""
import json
import struct

import numpy as np

FLOAT, USHORT, UBYTE, UINT = 5126, 5123, 5121, 5125
_NC = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
_DT = {FLOAT: np.float32, USHORT: np.uint16, UBYTE: np.uint8, UINT: np.uint32}


class Builder:
    def __init__(self):
        self.bin = bytearray()
        self.g = {"asset": {"version": "2.0", "generator": "HST-MODS psp2gltf"}, "scene": 0,
                  "scenes": [{"nodes": []}], "nodes": [], "buffers": [], "bufferViews": [], "accessors": []}

    def _list(self, key):
        return self.g.setdefault(key, [])

    def view(self, data, target=None):
        while len(self.bin) % 4:
            self.bin.append(0)
        v = {"buffer": 0, "byteOffset": len(self.bin), "byteLength": len(data)}
        if target:
            v["target"] = target
        self.bin += data
        self.g["bufferViews"].append(v)
        return len(self.g["bufferViews"]) - 1

    def accessor(self, arr, comp, typ, target=None, normalized=False, minmax=False):
        arr = np.ascontiguousarray(arr, dtype=_DT[comp])
        n = _NC[typ]
        count = arr.size // n
        bv = self.view(arr.tobytes(), target)
        a = {"bufferView": bv, "componentType": comp, "count": count, "type": typ}
        if normalized:
            a["normalized"] = True
        if minmax:
            r = arr.reshape(count, n)
            a["min"] = [float(x) for x in r.min(0)]
            a["max"] = [float(x) for x in r.max(0)]
        self.g["accessors"].append(a)
        return len(self.g["accessors"]) - 1

    def node(self, **kw):
        self.g["nodes"].append({k: v for k, v in kw.items() if v is not None})
        return len(self.g["nodes"]) - 1

    def add(self, key, obj):
        lst = self._list(key)
        lst.append(obj)
        return len(lst) - 1

    def image_png(self, png_bytes, name):
        bv = self.view(png_bytes)
        img = self.add("images", {"bufferView": bv, "mimeType": "image/png", "name": name})
        if not self.g.get("samplers"):
            self.add("samplers", {"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497})
        return self.add("textures", {"sampler": 0, "source": img, "name": name})

    def write(self, path):
        g = dict(self.g)
        while len(self.bin) % 4:
            self.bin.append(0)
        g["buffers"] = [{"byteLength": len(self.bin)}]
        js = json.dumps(g, separators=(",", ":")).encode()
        js += b" " * (-len(js) % 4)
        out = bytearray(b"glTF" + struct.pack("<II", 2, 12 + 8 + len(js) + 8 + len(self.bin)))
        out += struct.pack("<I", len(js)) + b"JSON" + js
        out += struct.pack("<I", len(self.bin)) + b"BIN\0" + self.bin
        open(path, "wb").write(out)


def read(path):
    """-> (json dict, bin bytes)."""
    d = open(path, "rb").read()
    jl = struct.unpack_from("<I", d, 12)[0]
    g = json.loads(d[20: 20 + jl])
    bl = struct.unpack_from("<I", d, 20 + jl)[0]
    return g, d[28 + jl: 28 + jl + bl]


def get_accessor(g, binary, i):
    a = g["accessors"][i]
    v = g["bufferViews"][a["bufferView"]]
    n = _NC[a["type"]]
    dt = _DT[a["componentType"]]
    arr = np.frombuffer(binary, dt, a["count"] * n, v.get("byteOffset", 0) + a.get("byteOffset", 0))
    arr = arr.reshape(a["count"], n) if n > 1 else arr
    if a.get("normalized"):
        arr = arr.astype(np.float64) / np.iinfo(dt).max
    return arr
