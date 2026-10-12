# R1 (second pass, 2026-10-11) — finish what the 2026-10-08 evening commits left open

The first R1 pass (2026-10-08 21:30) reverted/reopened B56, M1g, M1h, M1j, B40i and checked B63's one-process
handoff. This pass carries each reopened task to done or a specific blocker. One file per task.

## Outcome (2026-10-12)
- B40i, B56, B63: done (files 1–3).
- M1g: done. The five umpires are playable mods, and a test checks the roster and plays a rally with each
  (journal 2026-10-08-m1g-umpires/2-R1-FINAL).
- M1h: done. Fore noise, Out of Bounds spring chains (spring.rs), and the PSP games have no hair dynamics
  (journal 2026-10-08-m1h-hair-physics/2-R1-FINAL). The Out of Bounds solver is not the PS3 game's own: its
  parameters are read from their values, and its collision shapes aren't used.
- B54: blocked. Needs a close-up of the original's open mouth, or the user to name what looks wrong (file 4).
- M1j: blocked. The user has to choose between keeping each game's relative heights (mod women down to 0.62 m,
  plus Fore's chibis) and clamping every mod to HST's per-sex range (file 5).

## Not verified / not 1:1
- The details are in each file. Also seen while reviewing: in the agents' `--play` shots the court surface isn't drawn,
  even with no mods loaded (context/shots/m1g/disc_only.png). It wasn't looked into here.
