# 01 — call sites, app fixes, replay test

## Capture

`timeout 600 tools/pcsx2.sh python3 research/p3e5_draw_log.py 5 1470 context/p3d/shared_s05.bin shared 40`: the
P3e5 hook with the `shared` switch logs every draw on `*(gm+0x80)` with (vsync, ra, the word at sp+0x40). sp+0x40
holds the caller of 0x3553d0 (the shout) where ra is 0x35546c. 80 draws from vsync 7566 to 9035. The per-transition
totals equal rng_s05's (67 plus 4 placement draws at each of the two reseeds).

| vsync | ra | what |
|---|---|---|
| 7566, 8656 | 0x344a14 ×4 | 0x3449f0 placement (new-point message 6): one draw per player, unconditional |
| 7734, 8791 | 0x379138, 0x379180 | the serve toss: 0x3467b0 → 0x37af20 → 0x379080, the ball's two launch uniforms |
| strokes | 0x34792c | the fresh draw (+0x3b9c) |
| | 0x347a54 / 0x347b1c | the grade-2 / grade-3,4 shout chances |
| | 0x35546c (from 0x3479fc, 0x347aa0, 0x34797c, 0x347b68) | the shout key |
| | 0x379138, 0x379180 | the launch uniforms (0x37e950 → 0x379080) |
| 8350 | 0x347274, 0x3472b0, 0x3473f8, 0x347458, 0x347484 | mis-hit roll, framed check, wild lob (3) |
| 7886 | 0x35546c from 0x34bba4 | dive shout (program 3, 0x34afc0) |
| 8532 | 0x354bd8 ×4 | doubles team reactions |
| 8539 | 0x35546c from 0x3544b8 | post-point reaction voice (0x354050) |

The serve: when the server's byte +0x3ec0 is set, 0x3467b0 calls 0x37af20 (the toss) and returns before the
fresh/shout/mis-hit block. So the toss draws only the two uniforms. The serve hit is a branch-0 stroke through the
normal path (fresh, the strong-toss grade 1/2 shout, uniforms: 4 at 7784). Its aim (0x34c250) draws its two nudge
coins only when aimed within 1 m of the centre with the stick level, magnitude first then sign. 0x353000's
sideways coin is drawn only for a mistimed strong toss with a centred stick. No serve in the log draws a coin.

## Every call site

Callers of 0x19f5c0 with gm+0x80 (xref plus a scan for `lw a0, 0x80(gm)`):

| function | what | in a match? | app |
|---|---|---|---|
| 0x326960 | weather and wind schedule | setup and rematch (gm msg 0x1b) | `weather::schedule` (main.rs) |
| 0x19d270 | BGM variant pick (0–3, no repeat; bank 0x61/0x62) | setup, after the sparks | **not ported (P3d3)** |
| 0x343200 | hit-spark table rolls | 100 at setup (before the sound manager exists), then on the sound generator | sound side ported (P3e2); **setup's 100 not (P3d3)** |
| 0x3449f0 | placement: one draw per player each new point; doubles formation +0x13f4 (P3d2: by the 0x422fc8 words, staggered when the draw `% 100 < 20`) | every new point | draws ported (`placement_draws`); **formation roll not (P3d2)** |
| 0x3467b0 | stroke: mis-hit, framed, wild lob, fresh, shout chances (21 sites) | every stroke | `strike`, `timing::mis_hit/wild`, `sound::stroke_shout` |
| 0x379080 | ball launch uniforms (from 0x37af20 toss, 0x37e950 stroke) | every launch | `strike`; toss now `toss_draws` |
| 0x34c250 | serve aim nudge coins (conditional) | serves | `serve::target` (now lazy) |
| 0x353000 | second aim writer; strong-toss side coin | serves / net volleys | `serve::target`, `shot::aim` |
| 0x345ad0, 0x35edb0 | voice bank pick, AI object reseed | setup | `voice_bank`, `Rngs::new_ai` |
| 0x348a00 | singles new-point reaction (msg 0xe: 50% program-6 animation + voice) | singles only | **not ported (P3d3)** |
| 0x354050 → 0x3553d0 | post-point reaction voice (programs 7–10) for a player in the camera's close view | every point end | **not ported (P3d1)** |
| 0x354940 | doubles team reactions | doubles point end | `motion::team_reaction` |
| 0x3553d0 | shout key (all shouts) | – | `Voice::shout` |
| 0x3bf880, 0x3bfcb0, 0x3c13c0, 0x3c3310, 0x3c3770, 0x3c3ba0, 0x3d9c10, 0x3ea390, 0x3ea4f0, 0x3ebef0, 0x3ed080, 0x3ed140, 0x3ee240, 0x3ee790 | the one-player practice objects (feeder, coach, drills), built only when 0x422fa4 == 1 | practice only | no practice mode in the app |

Effects: `effects.rs` has no xorshift left (P3e2 moved the sparks onto the sound generator), and no effect draws
from the shared one in a match.

## App fixes

- `toss_draws`: the toss's two uniforms, drawn as the toss leaves the hand. The app made none.
- `serve::target` takes a coin closure and draws only where the game does. The app drew 3 coins on every serve.
  `aim.rs`'s `human_serve_aims` feeds the recorded coins in draw order (19/19 still exact).
- `placement_draws`: one draw per player after each new-point reseed (setup and `next_point`).

## Test

`shared_draws_like_the_game` (tests/rng.rs) replays the generator over all 1500 frames. A frame's draws show in
the next match sample:
- **Reseed:** `Rngs::new_point`, then 4 placement draws.
- **Launch (the live ball's +0x258 changes):**
  - A toss (a serve flag going up): 2 draws.
  - A stroke (hitter = the rally block's last hitter): `swing::mis_hit` on its branch, grade, kind, stamina,
    `launch_motion` and contact height, then `wild_aim` when framed, the fresh draw, `stroke_shout` with its
    chances, and `Voice::shout`.
  - Then the two uniforms, which must equal the ball's +0x258/+0x25c bit for bit.
- **Dive start (branch → 3):** its shout key.
- **Team reactions (+0x3db0 set):** `team_reaction`, which must give the recorded motions.
- **Reaction voice (+0x3d38 set):** one draw, taken from the recording (P3d1).

The generator lands on rng_s05's every frame: 2 reseeds, 2 tosses, 13 strokes, 1 dive, 1 team reaction and
1 reaction voice.
