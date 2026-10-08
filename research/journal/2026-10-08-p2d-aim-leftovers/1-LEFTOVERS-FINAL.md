# 1 — The aim's leftovers into the launch

## The aim core (0x34c250) leaves
- **+0x3ed4 nudge**: 0, then if |final x| ≤ 1 and the raw stick x == 0.0: draw 1 (bit 16 set → 5, else 10),
  draw 2 (bit 16 set → positive). Same rule the serve already had (P2c); for rally aims it was missing.
- **+0x3eca short-only** = the core's drop argument (0x34ccf0: the kind class 4, branches 1..3). The launch
  0x35b640 clamps the depth error to [−10, 0] with it, else [−10, 10].
- **+0x3edc held**: 0, unless no 0x20 button, branch 4, the smash class 0 (not △) and the mapped stick z × end
  < 0: then `(half / 1.5) · |stick z| / 2` (the raw stick z) and the stick's z offset is dropped.

## Where they go
- 0x35b640 (every launch): +0x3ed0 (side) = error (0 on grade 1/2) + nudge, clamped ±10; +0x3ecc short-only
  as above. Already in `swing::timing_launch`; `play/timing.rs` passed 0 / false.
- The smash (0x3467b0, branch 4): depth = (depth < 0 ? ×2 : depth) · +0x3ee0 − +0x3edc, side as above, into the
  common launch 0x37b110 (class 3), scattered along the shot line ×1.5 like any timing scatter.
- 0x35b030 for branch 4: depth error = bias (+0x3f98) + contact height steps off the ideal smash height
  (+0x13c4, `ReachStats::smash[1]`), ground-stroke thresholds (+0x1364/+0x1368), scale 1; no side error.
  +0x3ee0 = 1, or for a plain smash with |offset| > 1: `(clamp(|from z|, 1.5, 11.885) − 1.5) / 10.385 + 1`
  (the clamp bounds are data at 0x3fc8f0/8, read from RAM: 1.5, 11.885). Its mode (−body DWN, − the from-z
  distance term) only steers the blend, which the port's smash launch doesn't take (base smash table; the
  existing smash launch test is exact with it).

## Verification
- `human_aims`: every recorded rally aim's nudge, short-only byte and held depth bit for bit (the coins read back
  from the recorded nudge). Singles: 3 nudged, 1 held.
- `timed_smashes_like_the_game`: 28 smash launches (match_s05, lob_smash_s05, 1p3goodcpus, new_recording,
  human_smash_s04): scale (+0x3ee0), scatter, target and velocity bit for bit; 17 scattered, 2 held, 18 scaled.

## Singles ×0.45
`incoming` is Some(the *opponent's* slice was sweet), not P1's own timing. record_aim.py `AIM_INCOMING=sweet`
keeps only those; `AIM_STICKS` swaps the stick cycle for wide sticks (corners and full sides) so the aim reaches
the angle limit. See the recording below.

Only ✕ (kind 0) on a ground stroke, a volley's kinds 0..2 and the drop keep a finite angle; ○ (kind 1) and △ (the
lob) set it to 90°, so ×0.45 can never matter there. The first sweet-incoming run (all three buttons, wide sticks)
kept 2 aims, a ○ and a drop, neither past ×0.45's angle. `AIM_BUTTONS=cross` (new) with `AIM_STICKS="1,0;-1,0;1,-1;-1,-1"`
gave 2 ✕ aims off a sweet slice pulled in to x 0.898 off the sideline: the angle limit, which ×0.6 would leave wider.
Sweet incoming slices are rare (2 per 40000 frames in both runs).

Both runs (4 aims) are appended to `aim_singles.bin` (23 aims, incoming slices [4, 5], **2 off with ×0.6**; doubles
already had 3 of 8). The fixture is gitignored data: the extended file is also copied into the main checkout's
`context/fixtures/` so the merged `human_aims` (which now asserts this per fixture) passes there.

Commands (copy 5, slot 4 save, P1 human, singles faked through the player count):
```
AIM_SINGLES=1 AIM_INCOMING=sweet AIM_STICKS="1,-1;-1,-1;1,0;-1,0;1,1;-1,1;0.6,-0.8" tools/pcsx2.sh python3 tools/record_aim.py 4 sweet.bin 3 40000
AIM_SINGLES=1 AIM_INCOMING=sweet AIM_BUTTONS=cross AIM_STICKS="1,0;-1,0;1,-1;-1,-1" tools/pcsx2.sh python3 tools/record_aim.py 4 sweet2.bin 3 40000
cat sweet.bin sweet2.bin >> context/fixtures/aim_singles.bin
```
