# B260 — browser build: what runs while the tab is hidden

Scope as narrowed by the user (2026-10-14): a hidden tab **pauses the match** (resync on return is B299/B162); only
what isn't paused keeps running while hidden: store downloads and their queue, the lobby and its keep-alives.

- 1-TIMER-FRAMES-FINAL.md — rAF falls back to a timer while hidden, outside a match
