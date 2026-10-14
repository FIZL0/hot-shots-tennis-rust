# B276 — web "Out of Memory" mid-match during a wait

Task: the browser build quit with "Caught rendering error: Out of Memory" mid online match while the
"Waiting for Player to come back… (14 s)" pop-up was up. Find what grows, fix it.

- [1-DESYNC-REPORTS-FINAL.md](1-DESYNC-REPORTS-FINAL.md) — measurements, cause (a desync report saved again on every
  out-of-sync-and-back cycle), fix, what wasn't verified.
