# M1q — modded characters' animations too fast

## Cause
Mods draw only their own *reactions* (standard §6 `motions`: re_gu, re_di, re_gu_set, re_di_set); run, swings and
everything else are the donor's clips. Two places forced every own reaction into the donor clip's length:

- `open-tennis-mods tools/rerig/motions.py`: a source intro longer than `@FRAMES` (the donor's `.ANI2` length) was
  *squeezed* into it (`squeeze = lens[0]·60 / n`).
- `mods.rs own_motions`: keys were spread linearly over the donor clip (`t / span · end`), and a clip not exactly
  the donor's length was refused, so the drawn clip could never be longer.

Measured over the 87 packaged mods (348 reactions; source length from the source glTF, donor from its `.ANI2`):

| game | clips | squeezed (>1×) | median speed | max |
|---|---|---|---|---|
| Fore! | 96 | 85 | 1.94× | 3.25× (pc22 re_di 130→40 f; every re_gu_set 240→79 f, 3.04×) |
| Get a Grip | 60 | 39 | 1.06× | 2.75× (pc12 re_di 110→40 f) |
| Out of Bounds | 68 | 9 | — | 1.15× |
| Open Tee | 40 | 10 | — | 1.81× |
| Open Tee 2 | 84 | 9 | — | 1.15× |

The glTF time bases themselves were fine: the exporters write seconds (Get a Grip `.i3m` duration in s,
`psp2gltf/anim.py` resamples at 60 Hz; Fore `.ani` = HST ANI2, 80 ticks/frame; OOB `.mot` 4800 ticks/s), and the
loader ignored them anyway.

Donor motions on a mod were never retimed: `disc_motions` builds them exactly as for the disc character (same
lengths, ticks per frame), now pinned by the test.

## Fix
- `motions.py`: no squeeze; the clip is max(donor frames, intro) at 60 Hz (intro whole, then the loop / held last
  frame up to the donor's length as before). `package.py`'s `motions_why` says so; standard §6 updated (key times are
  seconds at 60 Hz; may outlast the donor's clip, not stop short of it).
- `mods.rs own_motions`: key tick = round(t·60)·ticks per frame; refuses a clip shorter than the donor's. Older
  packages (timed to exactly the donor's length) load unchanged.
- `character.rs`: `Motion::drawn` (+ `drawn_prev`), the gameplay clock's twin (same start, speed, loop, hold),
  wrapping by the drawn clip's length; `animate` draws a mod's own clip on it. Gameplay still reads `clock` and the
  donor clip, so timing and the sim are unchanged (for any other rig the twin equals the gameplay clock).
- Regenerated `motions.glb` + `motions_why` for all 87 packaged mods (scratch script repeating package.py's plan;
  not a full repackage, since another session had uncommitted `sizing.py` edits in open-tennis-mods). Check: all 348 clips
  are max(source, donor) frames, 148 longer than the donor's.
- Test `mods::own_motions_are_visual_only` now runs Emi and Phoebe: donor clips equal the disc donor's, own clip
  length = its glTF's seconds·60, Phoebe's longer clip runs to its own end while the gameplay clock holds the donor's.

## Not verified / not 1:1
- Source durations are taken from the files' own time bases (Get a Grip `.i3m` seconds, OOB 4800 ticks/s,
  Fore ANI2 80/frame); not timed against the PSP/PS3 games running (no emulator for them here). The OOB tick rate is
  already an assumption in open-tennis-mods notes/oob.md.
- A reaction longer than the time the post-point leaves on screen is cut by the next point's setup (it is no longer
  squeezed to fit); how much shows depends on the flow timeline. A fade *out of* an own clip samples it at the
  gameplay clock's time (the donor's end), not the drawn clock's: an 8-frame blend from a slightly different pose
  when the next motion loops. Not seen in `--play` (no side-by-side screenshot taken; presentation only).
