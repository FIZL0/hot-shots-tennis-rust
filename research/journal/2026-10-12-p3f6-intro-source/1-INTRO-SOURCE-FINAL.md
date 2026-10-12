# P3f6 — the intro's length from its source

## Watching the gates
`research/p3f6_intro_watch.py <slot> <route>` polls (±1 frame, not lock-step) the intro phase's gates with the match
frame gm+0x58: the timer `*(*(gm+0x84)+0x138)+0x3c`, the streamed-sound word 0x1e9f10 and the presentation flags
`*0x43b1d8 + 0xd94/0xd95`. `P3F6_STAGE=n` rewrites the court word 0x422f90 from the moment the menu sets it until the
match object exists, which reaches courts the Select Court list route doesn't (the list's labels aren't the court
numbers and wall-clock list moves drop presses). Copy 2: ✕ confirms, ○ backs out.

Court 6 doubles (slot 2, ✕ ×4, Start, ✕):
| frame | event |
|---|---|
| 0–3 | timer 3 → 0 |
| 150 | the sound word goes non-zero (the flyover's first move ends: c06 is 150 + 150) |
| 302 | d94 = 1: the camera shot is over (its object's +0x100 flag), the banner starts |
| 393 | d95 = 1 (banner animation at 90.0 of 90.0) → phase 2 the same frame |

So the intro is the camera flyover, then the banner. 0x324810 sets d94 once the timer is out, the stream is idle and
the shot's busy flag is clear (plus a check of the shot's +0x174 / 0x3694a0 that never held it here), and calls event
10 (the banner). The banner object (0x3a6050, made at event 6 of 0x3a46f0) sets d95 when its animation's time reaches
its length (0x38ee70: `length <= time`); its fade in/hold/out (15/60/15) runs alongside.

## The lengths' sources
- Flyover: `COURT/NN/CMN.XB` → `cam_cNN_00.dat` (B42's file): Σ record frames, a cut (a 0-frame record after the first)
  2 frames. Court 6: 150 + 2 + 150 = 302 = the d94 frame; court 5 (three segments): 100 + 2 + 120 + 2 + 130 = 354 = its
  d94 frame (watched).
- Banner: `AZUMA/INPANE/INPANE.XB0` → `CourtPreview/i_gamestart_00` (paths 0x41fb68/0x41fb80, loaded by 0x3a4b80).
  Its ANI's last key is tick 7200 at 80 ticks/frame = 90.0, the length read live at the animation (+0x2c).
- Ticks = flyover + banner + 2 (the banner starts at time 0 on the d94 frame and is seen done one frame after reaching
  90; the ticks count frames 0..that frame). Every court: 270+2 → 364 (1), 354 → 446 (5), 260+2 → 354 (11), 302 → 394
  (the rest) — all 22 recorded lengths.

## New recordings (P3F6_STAGE poke; the court loaded was checked by screenshot, `context/p3f6/`)
| court | mode | d94 | phase 2 | ticks |
|---|---|---|---|---|
| 3 | doubles | 302 | 393 | 394 |
| 10 | singles | 302 | 393 | 394 |
Also watched without poking, doubles: courts 2, 6, 7 (394), 5 (446), 11 (354) — the same as P3f2.

## Port
- `Rngs::intro_ticks_of(camera, banner)` (rng.rs): the flyover + banner + 2; `Rngs::intro_ticks_on_disc(iso, stage)`
  reads both files. play.rs's setup ticks the flare that many times (one line changed).
- `Rngs::intro_ticks(stage)` stays as the retail courts' values for callers without the disc (B42's `play/intro.rs`
  on main calls it), checked equal to the disc's for every court.
- Test `intro_ticks_from_the_disc` (tests/rng.rs): the disc-derived length = every recorded length (11 courts × both
  modes) and = `intro_ticks`. With the +2 made +1 it fails.

## Not verified / not 1:1
- P3f2's rand fixture (`context/p3f2/intro.bin`) is no longer in `context/`, so `intro_ticks_like_the_game` skips; the
  new test checks lengths, not the rand stream (P3f2 already tied the lengths to the stream).
- The watch polls over PINE (±1 frame); the frames above repeated exactly over runs and match the camera files'
  sums, so no lock-step capture was made.
- The extra d94 gate (the shot's +0x174 and 0x3694a0, under option bits 0x2f0653 / 0x2ef870) and the streamed
  sound never held the intro in any run; they aren't ported. A setting or a slow stream that delays them would
  lengthen the original's intro.
- Courts 3 doubles / 10 singles were reached by poking the court word, not from the menu (the list route doesn't
  reach them); the right court loaded (screenshots).
- The banner length is taken from the ANI only; its MTA/MOR end at the same tick (7200), which of them the game uses
  for +0x2c wasn't traced.
