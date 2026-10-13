# 1 — recover a silent peer (FINAL)

Native, `drop.sh` (three processes on one machine, doubles: host with 2 seats, guests a and b, relay off):

- **run1** (`context/b196/run1`): a killed, restarted with the same code → took its seat back, all three restarted on
  the same tick and hashed the same afterwards. Then killed again and nobody answered → the old end-on-timeout path
  (before the lobby change) ended cleanly on both.
- **run2** (`context/b196/run2`): first the same rejoin, in sync; then a killed again, host pressed Confirm → a bot
  took the seat; host and b hash the same at ticks 7200, 9000, 10800.
- Found and fixed: the rejoiner's 20 s handshake drop (see README).

The lobby return (added after these runs, with the 20/15/10 s waits) is covered by the in-process tests only.
A harness-only desync after the bot takeover (old seats back from the instant replay's copy) was found and fixed,
see README.

Unit/harness tests: `tools/check.sh -p hst` green.
