# B10: Jun's good hit went into the net

## Cause
The original launches a ground stroke as a **volley** (class 2: the hitter's `voly` tables, volley bounds, the
high-ball blend `high_blend`) when it is a topspin or slice (kind < 2), not framed, locked inside the service line
(|player +0x3ef8| < 6.4, the hitter's z when the swing locked) on a ball 0.6 m up or more (+0x3f44, the contact
height). The rally launch's branch-1 case calls the aim pull-in with class 2 and goes straight to the launch
(no counter check) when that holds; otherwise class 1. B2 had listed this as not ported.

The app knew the flag (`inside_high`, for the counter banner) but tested `at[1] >= 0.6`, and y is negative up, so it
never fired; and it never changed the launch class anyway. So those mid-court high balls flew off the stroke tables.

On the recordings' two such launches, the stroke tables give a lower launch:
- 1p3goodcpus vsync 21054, **Jun** (c2) kind 1 from z −5.07, ball 0.65 m up: game elevation 0.146 → crosses the net
  at 1.158 m; stroke tables 0.116 → 0.981 m, i.e. into the cord (0.914 m in the middle plus the 0.064 m radius;
  the crossing is off-centre). That is the reported bug.
- match_s05 vsync 32507, c5 kind 1 from z 5.86, 0.82 m up: game 0.262; stroke tables 0.218, never reaches the net.

## Fix
- `swing::high_volley` (the rule above); `play.rs strike` launches class 2 when it holds (z from `aim_from`, set at
  the lock; height = −ball y at the launch), and the counter check takes the same flag.
- `play/timing.rs launch`: the bounds are taken at the hit z moved back by the scatter, as the lookup's (the test
  always did; the app used the raw z, which only differs on a scattered hit).

## Verified
`timed_launches_like_the_game` now also checks every recorded ground stroke's class against `high_volley`: 168
branch-1 launches, the 2 class-2 ones included, 0 wrong (260 launches in all still exact).

## Not verified / not 1:1
- No new vpad repro on PCSX2 or in the app: the existing recordings already hold a Jun shot of exactly this kind
  (and it now launches as the game's, bit for bit, through the same lookup the test checks). Baseline sweet-spot
  strokes were already exact (43 recorded Jun launches).
- The height is −(ball y) at the launch; +0x3f44 differed from it by ~4e-6 on one recorded frame (0.816491 vs
  0.816495), which only matters for a ball within a few µm of 0.6 m.
