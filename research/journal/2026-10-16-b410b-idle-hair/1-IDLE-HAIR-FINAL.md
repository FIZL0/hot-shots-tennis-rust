# B410b — Out of Bounds hair jumpy at idle (after B410)

## Symptom
After B410, Sophie's pigtails and ribbons (and other OOB mods' spring chains) looked fine running, but jerked about
while she stood or idled (user 2026-10-16).

## Cause
Every spring link's mass was a free world-space point. When the link above turned, the link below lagged its own
origin all over again. Over a 10-link, 4 cm-a-link pigtail, a small idle bob (~3 cm) built up into a whip. A
per-frame log of the viewer idle (`--mod …/oob_pc02_sophie --motion 0`) showed links swinging between 0° and their
20–30° limit and never settling: up to 19° change per 60 Hz step while the head moved a few mm.
Also, OOB's stiffness, applied per 60 Hz step, rings at ~7 Hz with its 0.83 damping.

## Fix (`crates/hst/src/spring.rs`)
- `solve()`: the solver is a pure function over the skeleton's local poses (`swing` copies them in and out), so the
  tests run the game's code.
- Each mass is carried with its parent joint's world matrix between steps. Links below a chain's first ride fully with
  the link above. The first link keeps its world inertia, but its lag grows by at most 3° a step.
- Stiffness ×0.1 (`SOFT`).
- The user picked ×0.1 with the 3° cap on sight from 9 settings (stiffness ×1/0.3/0.1 × cap 3°/6°/none). Measured on
  the idle viewer: mean step jitter 0.25° against 2.99° before; sway (angle std) 1.08° against 4.16°.

## Checks
- `a_bobbing_head_does_not_whip`: 3 cm bob at ~3 Hz. Before: 86° off the pose, 38° in one step (fails). After:
  under 5° a step.
- `a_still_chain_settles`, `trails_then_settles_within_the_limit`: bounds loosened for the softer sag (rests ~44°
  off the pose at the chain's end).
- `tools/check.sh -p hst`: 468 passed.
- Side by side with task/B435's rewrite (unmerged), in CPU-vs-CPU matches with Sophie: the user chose this one.
  B435 is closed as superseded.

## Not 1:1
Out of Bounds' own solver is still not ported. `SOFT` and the 3° cap were picked by eye.
