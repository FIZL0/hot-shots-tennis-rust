# P17v4: what changes between two SW rebuilds of court 10 (in progress)

## Results so far
- There are two SW outcomes, A and C, and each one repeats exactly.
  - A = sw05: 38841 reds > 0x6f.
  - C = cap0: 39074 reds > 0x6f; it differs from A in 16277 reds and 4789 threshold bits.
- **Rebuilds from the same frame agree.** Every lock-step `research/p17v4_frame.py` rebuild gave A: frames 8, 9, 10, 11 and 200, some of them repeated, and a free-running variant too. The build frame doesn't matter.
- **The varying input is the 16 shadow-caster textures in GS VRAM**: PSMT4 128×128 at TBP 0x3308 + 0x20·i, the textures the build's receiver pass samples.
  - The receiver prims (vertices, TEX0, XYOFFSET) are identical in C (cap0) and A (d1).
  - The textures differ in 15909 texels.
  - The P17v sim run on cap0's VRAM reproduces C.
- **Nothing the game sends writes those textures.** I checked every dump (k0, x1, y1, cap0, nb): there are no GS transfers into 0x3300–0x3500.
  - Every frame, the game renders an R-only target (FBP 0x8c) and copies 0x1180 into **0x3208–0x32e8**, not into 0x3308+.
  - So the textures are whatever VRAM held after the save-state load.
- **The same load gives different VRAM.** Each load + dump with no build:
  - Lock-step load, dumped at frames 0, 1, 2, 4 and 8: always A.
  - Free-running loads in one session (f1, l1, f2, f3, l2, f4): f1 = C, all later ones = A.
  - Earlier: cap1 and the watch runs gave C; d/e gave A.
  - So C seems to come from the first load after the copy has run a while. Guess: queued GS work from before the load lands on the restored VRAM. This is unconfirmed.

## Next
1. Extract the VRAM from `sstates/…05.p2s` (`GS.bin` in the zip) and check whether the state's own VRAM is A or C.
2. If it is A, C is a PCSX2 load race. Fix it by loading twice, or by using the lock-step load. Then tick P17v4.

PCSX2 copy 5 ini: Renderer = 13 (SW) in both the main ini and the per-game ini. The per-game `Renderer = -1` was overriding the main ini, so earlier "SW" runs in this copy were HW. GSDumpSingleFrame is bound to F11.
