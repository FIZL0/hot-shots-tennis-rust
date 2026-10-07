#!/usr/bin/env python3
"""Minimal PCSX2 PINE client. Usage: pine.py [hexaddr ...]  -> prints game id/status, then u32 at each addr."""
import os, socket, struct, sys

SLOT = 28011  # PCSX2 default; non-default slots use pcsx2.sock.<slot>
OK = 0

def _loud(kind, err, tb):
    # any script using PINE: a crash says so in one line instead of a traceback that reads like partial success
    if issubclass(kind, KeyboardInterrupt): return sys.__excepthook__(kind, err, tb)
    sys.exit(f"CAPTURE FAILED: {kind.__name__}: {err}. Anything this run wrote is incomplete; don't use it. "
             "Is PCSX2 running with PINE on (tools/pcsx2-hst.sh) and responding?")
sys.excepthook = _loud

class Pine:
    def __init__(self):
        path = os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), "pcsx2.sock" if SLOT == 28011 else f"pcsx2.sock.{SLOT}")
        self.s = socket.socket(socket.AF_UNIX); self.s.settimeout(10); self.s.connect(path)  # a frozen PCSX2 fails, not hangs

    def _call(self, op, args=b""):
        self.s.sendall(struct.pack("<IB", 5 + len(args), op) + args)
        size, res = struct.unpack("<IB", self._recv(5))
        body = self._recv(size - 5)
        if res != OK: raise RuntimeError(f"PINE op {op:#x} failed")
        return body

    def _recv(self, n):
        b = b""
        while len(b) < n:
            c = self.s.recv(n - len(b))
            if not c: raise ConnectionError("PINE closed")
            b += c
        return b

    def _str(self, op): return self._call(op)[4:].rstrip(b"\0").decode()
    def read8(self, a):  return self._call(0, struct.pack("<I", a))[0]
    def read16(self, a): return struct.unpack("<H", self._call(1, struct.pack("<I", a)))[0]
    def read32(self, a): return struct.unpack("<I", self._call(2, struct.pack("<I", a)))[0]
    def read64(self, a): return struct.unpack("<Q", self._call(3, struct.pack("<I", a)))[0]
    def write32(self, a, v): self._call(6, struct.pack("<II", a, v))
    def read_block(self, a, n):
        """Read n bytes (multiple of 8) at a as one PINE batch message: one header, many read64 ops."""
        ops = b"".join(struct.pack("<BI", 3, a + i) for i in range(0, n, 8))
        return self._call_raw(ops)

    def read_regions(self, regions):
        """Read several (addr, n) blocks (n multiple of 8) in one PINE batch; returns their bytes concatenated."""
        return self._call_raw(b"".join(struct.pack("<BI", 3, a + i) for a, n in regions for i in range(0, n, 8)))

    def _call_raw(self, ops):
        self.s.sendall(struct.pack("<I", 4 + len(ops)) + ops)
        size, res = struct.unpack("<IB", self._recv(5))
        body = self._recv(size - 5)
        if res != OK: raise RuntimeError("PINE batch failed")
        return body

    def save_state(self, slot): self._call(9, bytes([slot]))
    def load_state(self, slot): self._call(0x0A, bytes([slot]))
    def version(self): return self._str(8)
    def title(self):   return self._str(0x0B)
    def game_id(self): return self._str(0x0C)
    def status(self):  return ["running", "paused", "shutdown"][struct.unpack("<I", self._call(0x0F))[0]]

if __name__ == "__main__":
    p = Pine()
    print(p.version(), "|", p.game_id(), p.title(), "|", p.status())
    for a in sys.argv[1:]:
        print(f"{int(a,16):08x}: {p.read32(int(a,16)):08x}")
