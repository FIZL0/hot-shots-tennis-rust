#!/usr/bin/env python3
"""Minimal PCSX2 PINE client. Usage: pine.py [hexaddr ...]  -> prints game id/status, then u32 at each addr.

Copies (HST_PCSX2=N) run at 1x (B24): recorders poll the running game with next_frame() and lose next to nothing
(a torn read is skipped and logged). HST_LOCKSTEP=1, for a capture that must have every frame: recorders
(Pine(step=True)) step the game one frame at a time instead: pause, FrameAdvance (pad R3, bound by
tools/pcsx2-hst.sh), read while paused; never torn, but ~10 frames/s at 2x and unwatchable. HST_REALTIME=1 launches
the copy at 0.25 for the wall-clock tools. Tools that poll by wall-clock without next_frame call require_realtime()."""
import atexit, fcntl, os, signal, socket, struct, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from vpad import send  # a copy's pad: pause, FrameAdvance

VSYNC, VSYNC_STALL = 0x1d5780, 10  # the game's vsync counter (every recorder's frame clock)
INST = os.environ.get("HST_PCSX2", "")  # parallel runs: this agent's own PCSX2 copy N (tools/pcsx2-hst.sh)
SLOT = 28011 + int(INST or 0)  # PCSX2 default; non-default slots use pcsx2.sock.<slot>
OK = 0
SPEED = os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), f"hst-pcsx2{INST}.speed")  # written by pcsx2-hst.sh

def _loud(kind, err, tb):
    # any script using PINE: a crash says so in one line instead of a traceback that reads like partial success
    if issubclass(kind, KeyboardInterrupt): return sys.__excepthook__(kind, err, tb)
    sys.exit(f"CAPTURE FAILED: {kind.__name__}: {err}. Anything this run wrote is incomplete; don't use it. "
             "Is PCSX2 running with PINE on (tools/pcsx2-hst.sh) and responding?")
sys.excepthook = _loud

LOCK = os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), f"hst-pcsx2{INST}.lock")

def _take_lock():
    # parallel agents share one PCSX2: one PINE user at a time, held until this process exits.
    # tools/pcsx2.sh holds it for a whole multi-step command and sets HST_PCSX2_LOCKED for its children.
    if os.environ.get("HST_PCSX2_LOCKED"): return
    global _lock
    _lock = open(LOCK, "w")
    try: fcntl.flock(_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        print("waiting for PCSX2 (another agent is using it)", file=sys.stderr, flush=True)
        fcntl.flock(_lock, fcntl.LOCK_EX)
    if hold := int(os.environ.get("HST_PCSX2_MAX_HOLD", 0)):  # as tools/pcsx2.sh: a hung run can't starve the others
        signal.signal(signal.SIGALRM, lambda *_: sys.exit(f"pine: held PCSX2 for {hold}s (the parallel-run cap); stopping"))
        signal.alarm(hold)

class Pine:
    def __init__(self, step=False):
        """step: lock-step on a copy (next_frame, settle, load_state below); else the game runs free as before."""
        _take_lock()
        path = os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), "pcsx2.sock" if SLOT == 28011 else f"pcsx2.sock.{SLOT}")
        self.s = socket.socket(socket.AF_UNIX); self.s.settimeout(10); self.s.connect(path)  # a frozen PCSX2 fails, not hangs
        self.lockstep = step and INST and os.environ.get("HST_LOCKSTEP") == "1"
        self.speed = speed = open(SPEED).read().strip() if INST and os.path.exists(SPEED) else None
        self.fast = fast = speed is not None and (float(speed) == 0 or float(speed) >= 1)  # 0 = unlimited
        self.resume()
        if self.lockstep: atexit.register(self.resume)  # leave it running, as found

    def pause(self):
        """Pause a copy (pad N's Guide toggles pause) and wait until it is."""
        if self.status() == "paused": return
        send(["press guide 50"])
        t = time.monotonic()
        while self.status() != "paused":
            if time.monotonic() - t > 5: sys.exit("pine: pad Guide didn't pause PCSX2 (is tools/vpad.py serve up?)")

    def require_realtime(self):
        """For tools that poll the running game by wall-clock: refuse a copy running at 1x or faster."""
        if self.fast: sys.exit(f"pine: this tool polls in real time, but this copy runs at speed {self.speed}; "
                               "restart it slowed: tools/pcsx2-hst.sh stop; HST_REALTIME=1 tools/pcsx2-hst.sh")

    def next_frame(self, last, delay=0.004):
        """The vsync of the frame after `last` (the first one not seen yet). Lock-step: pause and FrameAdvance one
        frame, so it is read whole while paused. Real time: poll until the counter moves, then wait `delay` s for the
        game's tick (a read right after the tick can still see last frame's values: B24 journal)."""
        if not self.lockstep:
            while (v := self.read32(VSYNC)) == last: pass
            time.sleep(delay)
            return v
        self.pause()
        v0 = self._u32(VSYNC)
        if v0 != last: return v0
        t = time.monotonic()
        send(["press r3 4"])  # FrameAdvance; R3 is no PS2 button in the copies' pad map
        while (v := self._u32(VSYNC)) == v0 or self.status() != "paused":
            if time.monotonic() - t > 5: sys.exit("pine: FrameAdvance (pad R3) didn't step PCSX2; was the copy "
                                                  "launched by this tools/pcsx2-hst.sh (it binds R3)?")
        return v

    def resume(self):
        """A paused copy (HST_PCSX2=N) is resumed with pad N's Guide button (TogglePause, tools/pcsx2-hst.sh);
        True if it was paused. The user's own PCSX2 is left alone."""
        if not INST or self.status() != "paused": return False
        send(["press guide 100"])
        for _ in range(30):
            if self.status() != "paused":
                print("pine: PCSX2 was paused; resumed it", file=sys.stderr, flush=True)
                return True
            time.sleep(0.1)
        sys.exit("pine: PCSX2 is paused and pad Guide didn't resume it (is tools/vpad.py serve up for this copy?)")

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
    def _u32(self, a): return struct.unpack("<I", self._call(2, struct.pack("<I", a)))[0]
    def read32(self, a):
        v = self._u32(a)
        if a == VSYNC and not self.lockstep: self._watch(v)
        return v

    def _watch(self, v):
        # ponytail: every recorder polls VSYNC until it ticks; when the match ends (or PCSX2 pauses) it never does,
        # and the recorder would spin forever holding the PCSX2 lock. Stop it instead; the output file is flushed.
        if v != getattr(self, "_vs", None): self._vs, self._vs_t = v, time.monotonic()
        elif time.monotonic() - self._vs_t > VSYNC_STALL and self.resume(): self._vs_t = time.monotonic()
        elif time.monotonic() - self._vs_t > VSYNC_STALL:
            sys.exit(f"pine: vsync stuck at {v} for {VSYNC_STALL}s (match over or PCSX2 paused); stopping")
    def read64(self, a): return struct.unpack("<Q", self._call(3, struct.pack("<I", a)))[0]
    def write32(self, a, v): self._call(6, struct.pack("<II", a, v))
    def read_block(self, a, n):
        """Read n bytes (multiple of 8) at a as one PINE batch message: one header, many read64 ops."""
        ops = b"".join(struct.pack("<BI", 3, a + i) for i in range(0, n, 8))
        return self._call_raw(ops)

    def read_regions(self, regions):
        """Read several (addr, n) blocks (n multiple of 8) in one PINE batch; returns their bytes concatenated."""
        return self._call_raw(b"".join(struct.pack("<BI", 3, a + i) for a, n in regions for i in range(0, n, 8)))

    def settle(self, regions, v):
        """read_regions until two reads agree within frame v (the EE runs while PINE reads); None if the frame
        ticks first (at full speed it can tick on every try: waiting for v again would spin forever). Lock-step: the
        game is paused, one read."""
        a = self.read_regions(regions)
        if self.lockstep: return a
        while True:
            b = self.read_regions(regions)
            if self.read32(VSYNC) != v: return None
            if a == b: return a
            a = b

    def _call_raw(self, ops):
        self.s.sendall(struct.pack("<I", 4 + len(ops)) + ops)
        size, res = struct.unpack("<IB", self._recv(5))
        body = self._recv(size - 5)
        if res != OK: raise RuntimeError("PINE batch failed")
        return body

    def save_state(self, slot): self._call(9, bytes([slot]))
    def load_state(self, slot):
        # lock-step: pause right after, so captures start a frame or two after the state. Not before: a load into a
        # paused copy leaves the game's vsync counter 2 behind its state for good (B24 journal)
        if self.lockstep: self.resume()
        self._call(0x0A, bytes([slot]))
        if self.lockstep: self.pause()
    def version(self): return self._str(8)
    def title(self):   return self._str(0x0B)
    def game_id(self): return self._str(0x0C)
    def status(self):  return ["running", "paused", "shutdown"][struct.unpack("<I", self._call(0x0F))[0]]

if __name__ == "__main__":
    p = Pine()
    print(p.version(), "|", p.game_id(), p.title(), "|", p.status())
    for a in sys.argv[1:]:
        print(f"{int(a,16):08x}: {p.read32(int(a,16)):08x}")
