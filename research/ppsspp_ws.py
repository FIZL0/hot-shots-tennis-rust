#!/usr/bin/env python3
"""A minimal client for PPSSPP's websocket debugger (`PPSSPPHeadless --debugger=PORT`), no dependencies.

    ws = Ws(port); ws.call("cpu.resume"); ws.call("input.buttons.press", button="cross", duration=4)

`call` sends {"event": name, ...} and returns the first reply of that event (other events, e.g. "log", are kept
in `ws.events`). Used by B509 to drive Get a Grip to a match and log its sound calls.
"""
import base64, json, os, socket, struct


class Ws:
    def __init__(self, port, host="127.0.0.1"):
        self.s = socket.create_connection((host, port), timeout=30)
        key = base64.b64encode(os.urandom(16)).decode()
        self.s.sendall((f"GET /debugger HTTP/1.1\r\nHost: {host}:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
                        f"Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n"
                        "Sec-WebSocket-Protocol: debugger.ppsspp.org\r\n\r\n").encode())
        head = b""
        while b"\r\n\r\n" not in head:
            head += self.s.recv(1)
        if b" 101 " not in head.split(b"\r\n")[0]:
            raise OSError(head.decode(errors="replace"))
        self.events = []

    def send(self, obj):
        data = json.dumps(obj).encode()
        n = len(data)
        hdr = bytes([0x81]) + (bytes([0x80 | n]) if n < 126 else bytes([0x80 | 126]) + struct.pack(">H", n) if n < 65536
                               else bytes([0x80 | 127]) + struct.pack(">Q", n))
        mask = os.urandom(4)
        self.s.sendall(hdr + mask + bytes(b ^ mask[i % 4] for i, b in enumerate(data)))

    def _exact(self, n):
        b = b""
        while len(b) < n:
            c = self.s.recv(n - len(b))
            if not c:
                raise EOFError
            b += c
        return b

    def recv(self):
        msg = b""
        while True:
            b0, b1 = self._exact(2)
            n = b1 & 0x7F
            if n == 126:
                n = struct.unpack(">H", self._exact(2))[0]
            elif n == 127:
                n = struct.unpack(">Q", self._exact(8))[0]
            payload = self._exact(n)
            op = b0 & 0x0F
            if op == 9:  # ping
                continue
            msg += payload
            if b0 & 0x80:
                return json.loads(msg)

    def call(self, event, **args):
        self.send({"event": event, **args})
        while True:
            m = self.recv()
            if m.get("event") == event or (m.get("event") == "error" and m.get("ticket") is None):
                return m
            self.events.append(m)
