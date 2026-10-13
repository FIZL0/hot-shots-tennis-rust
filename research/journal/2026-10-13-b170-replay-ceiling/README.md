# B170 — indoor court ceiling missing in replays (FINAL)

## Original
The original draws the hall's ceiling (beams, lamps) in instant replays on stage 7. Replay pending was caught over
PINE, saved to scratch slot 8, then load + delay + screenshot sampling: `context/b170/orig2/t3.50.png` is a low
close-up looking up at the drawn ceiling.

## Cause
B157's guard band (`gs.wgsl` `GS_GUARD`). The ceiling is in stage 7's hole model (`hole/extra_h01_s1111.mdl`, found
with HST_SKIP). From a low replay eye looking up, its big triangles have vertices with w ≤ 0 or outside the guard
band, so the port dropped them and the sky showed through (`context/b170/eye/r_on.png`). With the guard off, the
ceiling is drawn (`r_off.png`). Skipping extra_h01 with the guard off brings the sky back (`r_skiphole.png`).
Not a near/far-plane or bounds problem (B169's cause).

## Fix
`GsMaterial::for_court_batch`: the hole model gets no guard band, so the GPU clips its triangles. Every other court
model keeps it.

## Check (`context/b170/eye/`)
- `h_replay.png`: the replay view with the ceiling drawn, like the original's t3.50.
- `h_s07.png`: stage 7 match view, w-house2 still hidden (as B157 `orig_s07.png`).
- `h_s08.png`: stage 8 match view, no near foliage (as B157 `cmp_s08.png`).

## Not verified / not 1:1
- Why the original clips this node and doesn't drop it is unknown. B157 inferred "drop any triangle outside the
  band" from one match-view GS dump. Exempting the hole model is a by-eye rule (rendering only has to look right),
  not the microcode's per-node classification.
- Only stage 7 replays and the stage 7/8 match views were checked. Other courts' hole models no longer get the guard;
  B157 saw no guard effect on stages 1–6, 9–11, so no change is expected there, but this wasn't re-shot.
