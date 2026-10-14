# B199 — swing trail divide by zero

## What the game does

The trail update's fade has two integer `div`s: `(length − life)·128 / trunc(length·0.3)` and `life·128 / trunc(length·0.4)`.
Disassembled from GAME.BIN, both are bare `div $zero` + `mflo`, with no `teq`/`break 7` check, so a divide by 0
doesn't trap. The R5900 leaves LO at 1 for a negative dividend and −1 otherwise (PCSX2 and `hst-oracle`'s
`div` agree). Ported as `effect::ee_div`.

## When it happens

1-, 2- and 3-frame trails never divide by 0. Length 1 ends on its first tick. Length 2 ends before it has two
samples. Length 3 is 128 on its one drawn frame (rise 0 isn't reached, fall 1 isn't under life 1). So the task
line's guess was wrong.

A 0 is reached only by a trail started with **≤ 0 frames**:
- its `life` goes negative and never hits 0, so it lives on until the motion changes;
- rise or fall truncates to 0 with a negative dividend → alpha 1 (+ 64 × racket speed).

The app starts one after a rollback. `effects::SwingTrail.seen` isn't in the snapshot, so a restored motion looks
like a new swing, and `start` gets `(clip length − sampled) / speed`, which is ≤ 0 when the clock is at or past
the clip's end. B197's `.max(1)` guard is replaced by the game's quotient.

That trail then has reach = life/length ≥ 1, so the game's `powf(1 − reach, 0.5)` gets a negative base. Its y = 0.5
sqrt shortcut only applies to x ≥ 0. A negative base with a fractional exponent takes fdlibm's `(x−x)/(x−x)`, which
is 0/0 = +max on the EE. The angle π·max falls outside ±π, so width and reach keep their values. `libm::powf`
now ports that branch instead of asserting. `ribbon`'s spline index is clamped, because the reach (±max) puts
the sample far outside the buffer; the game reads past its buffer there, draw only.

Other integer divisions in hst-sim were checked: the rest are `%` of RNG rolls by data/table counts, none by a
frame length.

## Test

`crates/hst-sim/tests/trails.rs` `short_trails`: trails of 1, 2, 3, 0, −1 and −10 frames tick with a racket
moving 0.5 m a frame. It checks live/alpha per tick (160 for 3 frames, 33 = 1 + 32 for 0 and −1, capped at 255 for
−10 via −1536 / −4) and that `ribbon` draws without a panic. `trails_s05` (the 878 recorded frames) still passes
bit for bit.

## Not verified / not 1:1

- **The ≤ 0-frame alphas weren't checked against the running game.** The game never starts such a trail in a
  normal match, so it can't be recorded. The values come from the disassembly + the EE div/div.s rules that PCSX2
  implements.
- **No Bevy-level rollback test.** The panic needs a restore across a swing's start, and that comes down to
  `Trail::start` with ≤ 0 frames, which the sim test covers. `play/rollback.rs` is unchanged.
- **The ending frame differs (known, unchanged).** On the frame a trail ends, the game still samples and computes
  alpha/reach/width (not drawn); the port returns early. This is gameplay-neutral.
- **The ribbon of a ≤ 0-frame trail** is clamped to its samples. The game reads garbage past its buffer there.
