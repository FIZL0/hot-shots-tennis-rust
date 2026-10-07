# N3d4: gallery (FINAL)

## Bank
Slot 6, loaded by the match setup `0x322d90`: `0x19b6b0(snd, 6, v << 4 | court)` with v = `0x19d270(snd, −1)`, a
draw of 0..3 without repeats until all four have played (flags at snd +0x1d0..+0x1d3). `0x19acd0` slot 6:
archive `SND/COURT/C_SND{court:02}{A if v < 2 else B}.XB0`, bank `data/sound/VOICE/GALLERY/gal{sg|dv}{court:02}{a if v
even else b}.hd` (sg in A, dv in B). hits_s05: `galsg10a`, slot 4: `galsg04b`. Every court's four banks have
programs 0 (keys 0..2), 1 (0..1), 5 (0..2), 6 (0..1) and 10 (0/2/3; 1/4/5 on court 5): `gallery_banks_have_every_call`.

## Object (DAT_0043b1c0)
- Per tick `0x39b7a0` (vtable): nothing in the rain (court object +0x135 = 2/3) or while +0x1b61 is 0 (cleared by the
  camera objects at a match/set point); then `0x39dbd0`: with +0x864 off, count +0x868 down while +0x865 and then
  turn +0x864 on; with it on, the cheer `0x39cf80`; always the reactions `0x39d240`.
- Cheer `0x39cf80`, when +0x870 counts to 0: program 10, key by the count +0x1b70 (0 → 0, 1 → 2, else 3; court 5:
  1/4/5), from stand table[+0x86c] = {0, 180, 45, 225, 90, 270, 135, 325}° at 0.9·128 = 115, not placed (mode 0:
  `sound::stereo(volume, angle)`, no falloff); then a play speed rnd·2⁻³²·(1.25 − 0.875) + 0.875; next stand
  (mod 8); next in (int)(40·(1 − 0.7·rnd)) ticks (70 on court 5). RNG: the shared MT19937 at *(*(gm+0x84)+0x154)+0x50.
- Start `0x39da50(obj, level, flag, event, chain)`: +0x864 = flag, count 0, stand rnd >> 16 & 7, +0x870 = 1; an
  event ≠ −1 instead holds the cheer (+0x864 = 0, +0x865 = flag), +0x874 = chain, +0x1b68 = 1 for event 4 or < 2
  (else 0x8c), +0x860 = event, +0x868 = +0x1b68 + (0x3c for event 1, else 0x1e).
- Point (`0x383350` msg 0x17, call = judge +0x426): fault/let (2/4) nothing. Game or set (DAT_004230b8 > 0): an
  error call (1/3/5/6) `(3, 1, 1, 0)` and the error count +0x57c + 1; else `(3, 1, 0 or −1, 0)` by `0x38d560`
  (a side has a player whose controller word 0x422fc8 + 4·i is below 0x20, i.e. a human — slot 5's four
  bots have 0x21; both or neither → applause) and the count reset. Plain point: call 0 → count
  reset, event 0 (applause) for an ace off a strong toss, a return winner (+0x560 2), a smash winner (+0x560 3 via
  `0x38c550`), a dive winner or a passing ground/volley winner (`0x327000`), else `(0, 1, −1, 0)` — the cheer at
  once; calls 1/3/5 `(3, 0, 1, chain)`, chain every second error in a row; call 6 `(3, 0, 1, 0)`.
- Reactions `0x39d240` (more than one player): +0x1b68 down to 0, then event 0: program 0, key (last + 1) % 3, from
  stands 0, 3, 6 (0/225/135°) at 0x4c; 1: program 1, key toggling, same stands; 5/6: programs 5/6 (key % 3 / toggle)
  from a random stand at 0x66. Chains 0 → 5, 1 → 6 after 0x3c ticks (0xa0 on a game). The last keys reset at msg 6
  only. Event −1 with courts 1/4/6: program 9 when a player runs near the stands; 3/4 → 7/8/9: the match end.

## Proof
`gallery_matches_the_game` (hits_s05, drives `sound::Gallery` from the recorded score and last-hit branch): point 1
(smash winner) applause program 0 key 0 at 981 from 0/225/135° at 76 exactly, then the cheer from 1014 (the model
1012: 2 ticks late, the scoreboard's tick-31 stall); points 2/3 cheer at T+1 exactly. Every cheer: program 10 at
115, keys 0, 2, 3, 3…, stands in order, 12–40 ticks apart, scale word 0xe00..0x1400 (0.875..1.25), L/R from
`stereo(115, stand)`; each cheer keys its tone again 18 ticks later (the sequence). No cheer after the next serve
starts (`Gallery::hush`): nothing in the code found stops it, so this is measured. Random parts (stand, interval,
pitch) are checked by rule — the game's MT state is not recorded.

## App
`sound::reaction` / `favoured` / `Gallery`; `play.rs` starts it at the verdict, steps it each tick, hushes it at the
next serve and change of ends; `audio.rs` loads one of the four banks (picked by the clock) and plays by bearing
(`Sound::play_toward`). Left out: the match-start cheer, the match-end ceremony, program 9 near the stands, rain
silence, and the plain-point winners other than smashes (aces, return/dive/passing winners).

## Voices for every character (user note on N3d)
`hits_s04.bin`: `tools/record_sound.py 4 1800 … hits` from slot 4 (court 4, doubles, characters 6, 4, 3, 11 at
0x422fa8.., banks `dvv_vc{03,04,06,11}a` of `spu_s04.csv`), player 1 driven by ✕ every 0.8 s. `shouts_match_the_game`
now runs on both recordings and checks every player against their own character's bank: in slot 4, 3 certain and
1 chance stroke shouts, 6 whiff shouts (player 1's on `dvv_vc06a`), all four players on their own banks. Whiffs
without a shout are re-swings within 60 frames (muted, P8a) or swings with no ball for the player. Pitfalls: the
game's vsync counter stops outside a match, so a recorder asked for more frames than the match has never ends (main's
`pine.py` now stops it after 10 s) — keep presses time-bounded, or they drive the menus into a new game; writing
0x21 to the controller words (0x422fc8 + 4·i) mid-match does not hand a human to the CPU.
