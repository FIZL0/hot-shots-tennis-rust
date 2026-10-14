# B286: footstep effects too bright — FINAL

## Cause
`play/foot_fx.rs` passed the game's colours (0..128, `v / 128`) straight in as vertex colours. Bevy treats vertex
colours as linear and multiplies them with the sRGB-decoded texture, so every colour displayed as if gamma-encoded
again: court 10's dust (101,85,55) showed nearly white-cream, its footprints (25,22,20 → 0.20) as ~48 % grey instead
of near-black. The GS modulates in the framebuffer's (gamma) space, texture × colour / 128.

## Fix
`lin()` converts each channel sRGB → linear before it becomes a vertex colour: puff dust colour, footprint colour,
and (same file, same bug) the replay debris blade/clod colours and the clod shadow grey. Alpha unchanged. Sim untouched
(no RNG/timing effect).

## Check (by eye, PLAN: rendering only has to look right)
- Original: PCSX2 copy 2, slot 5 (court 10, ruins), screenshot bursts (`context/b286/run1`, `run2`); a running
  player's puff is a faint tan patch with a dark print beside it (`run2/girl.png`).
- Remaster `--play --stage 10` (HST_AUTOPLAY): before (`context/b286/hst2/feet.png`) a pale whitish blob; after
  (`context/b286/hst5/feet.png`) a faint tan puff and dark prints, like the original.
- Numbers on court 10 grass (~85,95,45), puff at alpha ≈0.42 (peak 96/128 × texture alpha): original GS blend
  ≈ (133,124,74); old port ≈ (166,146,99); fixed ≈ (147,133,80).

## Not verified / not 1:1
- Other courts: only court 10 checked against the original (slot 5 is the only bot-only state; other courts need a
  human-driven match). The fix is the same colour conversion on every court.
- Bevy blends in linear space, the GS in gamma: a light puff over dark grass stays a little brighter (~+14 on
  court 10, see above). Engine-wide (bounce puffs, all effects), left as is: within by-eye tolerance.
- Shots aren't frame-matched (`--shot-at` is wall-clock), compared puffs from different moments.
- Replay debris (clods/blades) colour change not compared with the original (rare dive replays).
