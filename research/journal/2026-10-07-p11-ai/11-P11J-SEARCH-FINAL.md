# P11j: the AI's contact searches (final)

The tool `tools/record_ai_search.py <slot> <frames> <out> [keep 1 in N]` hooks the reach search 0x360150 (tag 1) and
the tiered search 0x360910 (tag 2), plus the AI generator's draw counter, the same way `record_ai_serve.py` does.
Each record carries:
- the call's arguments, including the stack ones;
- the AI object, the character record's reach block and the player's running state;
- the stamina floor, the strong side and the predicted path;
- the seen flags, before and after the call;
- the four out values.

The test `crates/hst-sim/tests/ai_search.rs` replays every call through `ai::Searcher` and `ai::Runner`. It checks:
the found flag, the stand spot (bits), the ball, the run frames, the index, the seen flags and the draw count.

| Fixture | Match | Reach searches (found) | Tiered searches (found) |
|---|---|---|---|
| `ai_search_s05.bin` | slot 5, doubles, 1848 frames | 16 (8) | 191 (8) |
| `ai_search_singles.bin` | slot 8 (`bots_singles`), 2248 frames | 11 (0) | 187 (11) |

All four tiers are covered, as are the no_reach and no_height flags and the reach search's body flag. Every call matches bit for bit.

## The port (`hst_sim::ai`)

- **`Runner::frames_to`** (0x35bc50), the run estimate. It returns 9999 when the spot is on the AI's own side, when
  |x| > 8.685, or when |z| is outside 1.5..17.885. Otherwise it simulates the run:
  - acceleration up to agility;
  - then the fresh-stamina block, (59 − tick) + (stamina − 10)·60 frames;
  - then frame by frame with the stamina drain, with the speed recomputed once drained.
- **`Searcher::reach_search`** (0x360150): ground stroke, volley or body shot. The score is ball y − stroke height,
  and the higher score wins.
- **`Searcher::tier_search`** (0x360910), with these bands:

  | Tier | Height (×H) | Reach (×R) |
  |---|---|---|
  | 0 | 0.9–1.1 | 0.9–1.1 |
  | 1 | 0.7–1.3 | 0.7–1.3 |
  | 2 | 0.3–1.7 | 0.3–1.3 |
  | 3 | 0–3 | 0–1.3 |

  - The score is |h − y| plus two penalties, each added only when above 0: 0.05·(|z| − 11.885)/0.5, and the same
    for |x| − width. The lower score wins.
  - Flagged entries are skipped before the 2-bounce exit. The flag is set once the distance is in range.
- Both searches:
  - The run budget is index − AI+0x30.
  - The output index is relative to AI+0x30.
  - The stand x is msub/madd(x, reach, side); the stand y is the ball's y.

## Recorder pitfalls (fixed)

- **Stub offsets.** The stubs address DATA through `t4 = DATA & 0xffff0000`, so every DATA word needs `+ (DATA &
  0xffff)`. A keep counter written at a bare offset incremented an instruction in the counter stub, so PCSX2 hung or
  crashed after a few hundred calls.
- **State load.** `load_state` lands late. Patches written within ~0.3 s can be overwritten by the state's own RAM,
  which holds stale stubs from older tools. The tool now waits until the loaded game has run 5 frames, then patches
  while paused.
  - `record_ai_serve.py` and the others still sleep 0.3 s; their runs happened to work.
- **Reading the buffer.** A read of more than about 1 MB in one PINE batch times out, so the tool reads 64 KB at a
  time.
- **Free RAM.** 0x1d00000–0x1feb000 is never written by the game in 4000 frames of slot 5, so 0x1e00000–0x1f80000 is
  safe.
