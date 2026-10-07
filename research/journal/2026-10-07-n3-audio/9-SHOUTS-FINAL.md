# N3d1: stroke and dive shouts (FINAL)

## Paths
- `0x3553d0(player, program, lo, hi, slot)`: keys lo..hi minus the one stored at player +0x3b64[slot] (slot < 0: no
  memory), picks `(rand >> 16 & 0x7fff) % n` (game RNG `0x19f5c0`, state gm+0x80), stores it, plays via `0x3554c0`
  (bank slot player +0x12b8 + 1, at +0x3d70, 0x80). No candidate → no play.
- Stroke launch (`0x3467b0`), after the mis-hit roll (bVar5 = framed +0x3f06 or dull +0x3f0c was set):
  - +0x3b98 = 0, +0x3b9c = a fresh random; branch +0x3ec1 == 3 (dive) → nothing here.
  - mis-hit → program 2 keys 0..2 (0..1 with 3+ players), slot 1; +0x3b98 = 1.
  - branch 0 (serve): `0x35d1f0(+0x3ea0) == 1` (strong toss) and grade +0x3ee8 1 or 2 → program 0 keys 0..2|1, no
    slot; +0x3b98 = 3. Any other serve: nothing.
  - branch 4 (smash) → program 0, same.
  - branches 1/2: |offset +0x3fa0| < 2, or grade 2 and roll%100 < 40 (20 with 3+ players) → program 1 keys 0..4|1,
    slot 0; +0x3b98 = 2. Else grade 3/4 and roll < 40|20 (halved for character +0x12bc 9 or 10) → program 2.
- Dive start (`0x34afc0`, after the contact search, +0x3ec1 == 3) → program 3 keys 0..2|1, no slot.
- `0x3552c0` (shout for a quiet stroke: program 0 key +0x3b9c % 3 or program 1 % 6) is only called from `0x371270`
  under gm+0x344 (the close-up camera) with 2 players: left out with the camera.
- Voice bank load (`0x345ad0` → `0x19acd0`): `SND/VOICE/PC/PC{char:02}VCE{n}.XB0`, n = 0/1 singles a/b (`SGL/sgv_vc`),
  4/5 doubles (`DBL/dvv_vc`), +2 in game mode 7 (c/d); a/b 70/30 at random, with rules for same-character players.

## Proof
`shouts_match_the_game` (hits_s05.bin, doubles, the four `dvv` banks at their `spu_s05.csv` addresses): every launch
keys `stroke_shout`'s program on the hitter's bank that frame or the next — 11 certain, 4 taken chances, never one
that cannot happen — and both dives key program 3 as they start; one bank per player (p0 vc00a, p1 vc02b, p2 vc01b,
p3 vc05b); every key ≤ 1 and programs 1/2 never repeat a player's last key (p0 1,0; p3 0,1,0; p2 0,1,0).

## Left (N3d2)
- Program 4 at 3053 (p3, a missed swing at 3044, offset 999): `0x350840` (whiff animation 0x27/0x2a, +0x3f96 = 1),
  program 4 keys 0..(players == 2), unless +0x4088 or +0x3f04.
- Programs 6–10 after points (1029 p1 (7,1); 1809 p3 (6,3); 2740 p0 (6,4); 3012 p1 (6,3)): `0x354050` plays 7–10 by
  reaction animation +0x3db0 (0x2c..0x2f), stored at +0x3d38; `0x348a00` program 6 (states 0x10, 0xe); `0x3522b0`
  program 6 key 2; `0x355350` (from `0x3d5b20`) replays logged voices (gm+0x55 == 3).

## App
`audio.rs voice_bank` loads each player's bank (variant a/b 70/30 at random) into `VoiceBanks`; `play.rs` shouts at
the launch and the dive start through the whoosh queue (at the player), keys from the app's own RNG.
