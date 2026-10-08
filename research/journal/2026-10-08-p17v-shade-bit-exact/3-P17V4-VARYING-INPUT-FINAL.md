# P17v4: the varying input is the shadow-caster texture pool left by the save-state load

## Answer
- **The varying input is GS VRAM at TBP 0x3308–0x34ff.** These are the 16 PSMT4 128×128 shadow-caster textures that the build's receiver pass samples (TEX0 0x2007810ddd40b308 + 0x20·i).
  - The game never rewrites this pool while it plays.
  - So a rebuild uses whatever the pool held right after `load_state`.
- **Tree animation and read-back timing don't matter.** The build frame doesn't either.
- **Rebuilds from the same frame give the same reds:**
  - Lock-step rebuilds at frames 8, 9, 10, 11, 200 and 8 again (h4), plus free-running rebuilds (h1–h3, some after 60–90 s of idle play), all equal sw05 bit for bit: 0 reds differ, 38841 reds > 0x6f.

## The two outcomes
| | A = sw05 | C = cap0 |
|---|---|---|
| reds > 0x6f | 38841 | 39074 (16277 reds and 4789 threshold bits differ from A) |
| texture pool 0x3308+ | **equals the pool in slot 5's own `GS.bin`** | differs from it in 15909 texels; found in no save state of any copy |
| receiver prims (vertices, TEX0, XYOFFSET) | identical | identical |
| P17v sim on that dump's VRAM | — | reproduces C |

So A is the correct rebuild of the state, and C is a corrupted pool.

## Evidence
- **The game never writes the pool.** Six dumps were checked (k0–k8, x1, y1, cap0, nb): no GS transfer lands in 0x3300–0x3500.
  - Each frame the game renders an R-only target (FBP 0x8c, FBW 4, FBMSK 0xffffff) and copies 0x1180 → 0x3208…0x32e8 as PSMT4 128×128. Those are the next 8 textures, not the receivers' 16.
- **What the pool held after each load:**
  - Lock-step loads: always the state's own (dumps at frames 0, 1, 2, 4 and 8 after the load).
  - In one session of free-running loads (f1, l1, f2, f3, l2, f4), only the first, f1, had the C pool.
  - In earlier sessions, C showed up in cap1, the watch runs and x1. Those were all capture-style flows: free-running load, countdown written while running, F11 dump.
  - So an occasional load leaves a pool that isn't in the state.
- **Not idle time, not MTVU.** h1 and h3, run after 60–90 s of free play, were A. One C run (m_nof2) happened with MTVU off.
- The exact PCSX2-side trigger is not pinned down. It is outside the game and outside the port.

## How to guard against it
- Rebuild with `research/p17v4_frame.py` under HST_LOCKSTEP=1.
- For a dump-driven replay, first check the pool:
  `research/p17v4_pool.py <state.p2s> dump.gs` says whether the dump's pool equals the state's.
  - The cap0 dump fails this check. Earlier P17v sim comparisons against cap0 were made on the C pool. sw05.red/.map are the A outcome, which is what the state really gives.

## Notes
- **This copy's renderer setting.** In copy 5, the per-game ini (`gamesettings/SCUS-97610_72326E67.ini`) had `Renderer = -1`, which overrides the main ini. Copy 5 is now SW (13) in both inis, and GSDumpSingleFrame is bound to F11.
  - sw05 equals true SW output, so P17v's SW claims stand.
- **F11 doesn't always dump.** Once, right after a restart, F11 saved only a screenshot and no dump.
- **Files** (in context/p17v, not committed): h1–h4.red, k0–k8 / f1–f4 / l1–l2 / y1 dumps, state05.GS.bin.
