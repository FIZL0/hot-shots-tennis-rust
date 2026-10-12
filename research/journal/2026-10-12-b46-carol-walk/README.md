# B46 — Carol's walk after a point (B3 follow-up)

Status: blocked. No mismatch reproduced.

## Setup
The user's `run.sh` lineup: doubles on stage 4, chars 6/11/2/10, outfits 9/9/4/9. Carol (6, lefty) is human P1.
The same lineup is in the fixtures `1p3goodcpus.bin` (+ `_ee.bin`) and `1p3goodcpus2.bin`
(from `context/recordings/1p3goodcpus2.p2m2`). For the app, a temporary headless probe (since removed) built the
same match, held the stick in Post and printed P0's cmd id, body motion, pos, dir, target, turn way and reaction root.

## What the original does (fixtures, P0 in phase 4)
- She stands still, or runs with the stick, facing the way she runs. Early in Post the stick isn't camera-flipped.
- She can still press strokes (mode 2, motions 19/40/20/39).
- The team reaction (0x30–0x34, `re_pc00_*_dummy` paths) can start mid-run. It moves her backward from her facing:
  a first-frame jump of ~0.123, a small drift back, then ~1.37 m over ~120 frames.
- Bots also make long-way turns (270–360°) in Post. That is in the original too.

## What the app does (same inputs)
- Stick left in Post: she runs left facing −x, then reaction 0x33 carries her +x. The jump/drift/travel pattern
  matches numerically. `motion::reaction_root` is already bit-exact (test `match_reaction_root` on `match_s05` and
  `1p3goodcpus`).
- Facing: `player::turn` is exact on both matches, including lefty Carol (test `match_facing`).
- Clips co03/co04/co05: Bip01 only moves ~0.07–0.18 m in x. The Pelvis track is static. All the z travel is in the
  dummy path, so the walk is not translated twice.

## Tried and dropped
- Syncing `body.motion` to `cmd.id` before `body.step`. The game's +0x3df0 holds the stroke motion right after a
  stroke, while the app keeps the last run motion. It didn't change any observed turn, so I reverted it. The
  replay tests stay untouched.

## Not verified / not 1:1
- No visual side-by-side (`--shot` vs `tools/screenshot.sh`) of the reaction walk. Everything numeric matches, and
  without knowing which moment looks wrong there is nothing to aim a shot at.
- The `body.motion` ↔ +0x3df0 difference right after a stroke. No fixture moment showed it changing the turn. It
  would need a lock-step check over a stroke→run transition (e.g. `1p3goodcpus2` frames 3728–3742).

## Unblocks
A video or description of the wrong walk: which point end (won or lost, serve or rally), stick held or not,
camera close-up or not. Or a `.p2m2` of the same moment from the user's own play, so the app can be driven to it.
