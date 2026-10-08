# 1 — Longer recordings, incoming slices only

## Runs (slot 4, `AIM_INCOMING=1 tools/record_aim.py 4 <out> <n> <max frames>`)
- Doubles: 20 incoming aims in 53675 frames (~200 presses); the AI slices often enough in plain play.
- Singles (`AIM_SINGLES=1`): 5 incoming aims in 14653 frames. A first, longer singles run ended in CAPTURE FAILED
  (a bad PINE read at vsync 31716) and was thrown away; the clean re-run is the one used.

## Result (`human_aims`, all bit for bit)

| fixture | aims | sweet full-diagonal | incoming off-sweet | incoming sweet |
|---------|-----:|--------------------:|-------------------:|---------------:|
| aim_singles.bin | 19 | 3 | 4 | 1 |
| aim_doubles.bin | 30 | 7 | 10 | 8 |

- Doubles' ×0.45 matters: with ×0.6 instead, 3 sweet-incoming aims differ (14924, 25414, 28804).
- Singles' one sweet incoming aim (stick (−1, 0), → (4.115, −7.408)) doesn't reach the angle limit, so it passes with
  either factor. It meets the coverage rule, but only doubles actually tests the ×0.45 value.

## Left out: a second aim writer (gap)
- 4 doubles aims (vsync 35354, 35372, 61634, 61657) differ, all in quick volley exchanges near the net. Their targets
  ignore the stick: x ≈ ±0.04 and z at −11.385, −3.0 or −2.0.
- The original has a second writer of +0x3e90, 0x353000 (called from 0x351dd0). It aims from a fixed base,
  x = end·2.0575 (negated by the global 0x423050), z = (3.4/2 + 3)·end (2.4 when off-sweet), through the same
  compute 0x34c250, and sets +0x3f10..+0x3f1c nudges. `shot::aim` ports only the stick aim (0x34ccf0).
- Those 4 pairs were dropped from the fixture. Porting this second path is a separate task (P2c in PLAN).
