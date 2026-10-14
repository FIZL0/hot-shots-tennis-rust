#!/usr/bin/env python3
"""tools/instr.py TID SECONDS: user-space instructions a thread (a process's main thread: its pid) retires over SECONDS, from the
CPU's counter (perf_event_open; no perf needed): a measure of work that other load doesn't change (B345, web_check WEB_TRACE)."""
import ctypes, os, struct, sys, time
libc = ctypes.CDLL(None, use_errno=True)
def counter(tid):
    attr = bytearray(128)
    flags = (1 << 5) | (1 << 6)  # exclude_kernel, exclude_hv (enabled at once)
    struct.pack_into('IIQQQQQ', attr, 0, 0, 128, 1, 0, 0, 0, flags)
    buf = ctypes.create_string_buffer(bytes(attr), 128)
    fd = libc.syscall(298, buf, tid, -1, -1, 0)
    if fd < 0:
        raise OSError(ctypes.get_errno(), os.strerror(ctypes.get_errno()))
    return fd
def read(fd):
    return struct.unpack('Q', os.read(fd, 8))[0]
tid = int(sys.argv[1]); secs = float(sys.argv[2])
fd = counter(tid); a = read(fd); time.sleep(secs); print(read(fd) - a)
