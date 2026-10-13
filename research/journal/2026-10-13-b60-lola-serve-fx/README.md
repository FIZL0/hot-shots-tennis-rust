# B60 — Lola's serve FX (2026-10-13)

User (2026-10-08): Lola's serve FX look wrong; reopens B33's visuals. B33 assumed a special serve uses the clean-hit
effect set.

## Decompile

- The effect manager's hit event takes its kind from the shot launch: the same shot kind the special check uses.
  In RAM, Lola's slice serve has effect kind = 1 (slice).
- Impact start has no special-only or character-only visuals:
  - slice (1) and lob (3) are drawn at scale 1.0, the other kinds are scaled by speed;
  - the model is the kind's own, except a smash, which uses model 5;
  - the rest of the function is hit sounds and replay-only sound branches.
- Ribbon colours (`FLIGHT_COLOURS`, decoded big-endian as `effects.rs` does): slice is orange, topspin blue/purple.
  Glow textures as fixed in B6: slice and drop use the orange ○ (`impactef_top`), topspin and flat the purple ✕.
- Sound special:
  - on a serve with a strong toss and kind < 2: the swing offset kept per player at strike has |offset| < 2;
  - otherwise: the timing grade is 1..2;
  - the port's `shot::special(class, kind, strong, grade, offset)` follows the same rule.
- Lola (character 10) with a special serve keys her rush voice at 1.5× pitch, volume 0x80 (`sound::Flight::start`).
  Both original runs keyed this voice at scale 0x1800 (= 1.5×), as the port does.

So **Lola's special serve is the generic slice set**: orange slice impact burst, orange ○ glow, orange ribbon with
white wisps, slice sparks, plus the 1.5× rush. No Lola-specific visuals exist.

## Captures (`context/b60/`, PCSX2 copy 3, slot 8 → Lola serving)

| run | serve | result |
|---|---|---|
| `run1/` | topspin (✕ button), special | purple burst/✕: confirms the button↔kind mapping, not Lola-slice |
| `run3_circle/s00s.png` | slice, ~hit+40 | orange ribbon |
| `run4/s0s..s5s.png` | slice special, ~hit+10 | orange burst at contact, orange ○ ring, orange ribbon + white wisps, ♪ balloon |
| `run5`, `run51` | slice, sound ring dump | rush voice at 0x1800 in both (with and without offset poke) |
| port `k2s..k38s.png` | slice special, tick-exact `--shot` at hit+N (local hook, reverted) | `k10s` ≈ `run4/s0s` |

Side by side: `context/b60/cmp.png` (top: original run4 s0; bottom: port k10). Same elements, colours and placement.
By eye the port's ribbon core is slightly thinner and more saturated. Rendering only has to look right, so this
gets no follow-up task.

The grey ♪ balloon in `run4` comes from my poke of the per-player offset (the balloon then grades Note, not
Sweet). A clean special hit shows the pink Sweet ♪ in both games.

## Result

No bug was found in the current port: effects, textures, colours, placement and sound match the original for
Lola's special and normal slice serves. B33's "special = clean-hit set" assumption is wrong in its wording but harmless:
the visuals key off the shot kind alone, and the port draws by kind. The complaint probably predates B6's glow-texture
swap fix. No code changes.

## Not verified / not 1:1

- **Frame-exact timing of impact/ribbon:** PCSX2 F8 screenshots land ~38 frames late even at 0.05× speed. Only
  ~hit+10 and ~hit+40 were compared, by eye. The impact's lifetime stays as P9b measured (12–15 frames).
- **Volume/pan words of the sound-ring commands** were not decoded. Only the voice and pitch scale were compared.
- **Why the no-poke original run still keyed the rush:** the per-player strike offset (which the sound special reads)
  was not read directly. A different player word read 2 there. Both runs played the rush, as the port does for a
  special, but the strike offset value itself is unverified.
- **Court differs:** the port shots are on stage 10, the original on court 11. Only the effects were compared, not
  the backgrounds.
- **Ribbon width/saturation:** slightly thinner and more saturated in the port (by eye; not chased, rendering rule).
