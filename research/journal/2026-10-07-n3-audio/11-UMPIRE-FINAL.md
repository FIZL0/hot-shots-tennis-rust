# N3d3: umpire calls (FINAL)

## Bank
Slot 5, loaded by `0x19acd0` with param_3 5: `SND/UMP/UV{de:02}{a|b}.XB0` → `data/sound/UMPIRE/gag_vc{de:02}{a|b}.hd`
(c/d with param_5). de = DAT_002ef7de, the menu's umpire 0..4 (>4: random, `0x186e80`); b with DAT_002ef7df. Slot 5
has umpire 4 voice a (`gag_vc04a` at SPU 870656). Bank: program 0 keys 0..13 (score words), 1 keys 0..4 (line
calls), 2 keys 0..4.

## Object (DAT_0043b1c8)
- Play `0x3a23d0(obj, program, key)`: stops the last word (`0x1a03f0` / `0x1a0390`), plays `0x1a02a0(5, program,
  key, 0x80)` → `0x19be70` → `0x1ac380(1, seq, program, key, 0x80, 0, 0)`, not positional. The recorded level equals
  a positional play of 0x80 at bearing 0 inside full volume (`sound::stereo(0x80, 0)` = 90/90 with bank volume 115).
  Program 0 (and 2 keys 1/2) also sets +0xdc, the frames to the next word, from GAME.BIN: program 0
  `0x413e20 + lang·0x230 + de·0x70 + df·0x38 + key·4`, program 2 `0x414b40 + lang·0xc8 + de·0x28 + df·0x14 + key·4`
  (lang = DAT_002ef110, the region's language, 0 here; 11 uses row 5). Umpire 4 a: 46 51 43 46 0 0 0 0 0 0 60 0 0 0.
- Messages (`0x3a1880`, from `0x18b310` as the match phase gm+0x55 is entered):
  - 0x17, point over (phase 4): line call program 1 key call − 1 when the judge's call is not 0 or 6 (and
    DAT_002eefe8, the pause/overlay flag, is 0); event ≠ −1 also points the umpire's arm (`0x3a2340`, animation).
  - 0x19, sent by `0x326270` when the scoreboard's +0x192 is first set (end of its 30-frame pause): +0x193 = 1 and,
    for a plain point (DAT_004230b8 == 0), the score queue: deuce (0x316620) one word, key 9 or 13 when the deuce count
    (0x316624) is 2 and court (0x422f90) ≠ 5; advantage (0x316628) 10 then 11 if the serving team (server 0x42304c
    0/2 → 0, else 1) won the point (0x4230a8), else 12; tiebreak nothing; else the serving team's points (0..3) then
    8 ("all") if equal, else the receivers' points + 4. A game/set (event 1/2) sets +0xd5 when the tiebreak flag is
    on (the game that made 6-all).
  - 0xc change ends: program 2 key 3. 0xe next serve: program 2 key 0 when +0xd5 (+0xd6 under the close-up
    gm+0x344), then cleared. 6/0xc/0xe (0xe only when gm+0x54 ∉ {0,1}) clear the queue (`0x3a1610`).
  - Match over: `0x326270` plays program 2 key 4 once the scoreboard is up, waits for it to stop, then phase 5.
- Update `0x3a1c10` while +0x193: +0xdc −= 1; if the word still sounds and +0xdc > 0, wait; else play queue[+0x194]
  (program byte +0x1a0+i, key +0x198 + 4·(i if i < 2 else 0)). Program 0 moves +0x194 0 → 1, else ends the queue.

## Proof
`umpire_calls_match_the_game` (hits_s05, score from the recording's 0x423064 and rally flags): the six words
0/0 0/5, 0/0 0/6, 0/1 0/6 at frames 1014, 1058, 2290, 2334, 3132, 3181, i.e. 33 ticks after the score changes (34 in
this ring; 33 in sound_s05) and gap − 2 after the first word (44 = 46 − 2, 49 = 51 − 2), each at the level above;
nothing else from the bank. The −2 is measured, not explained (the words are shorter than their gaps, and nothing
keys off). No fault, change of ends, tiebreak or match end in the recordings: programs 1 and 2 follow the code only.

## App
`sound::score_call` / `Umpire` (word queue), `line_call`, `CHANGE_ENDS_CALL`, `TIEBREAK_CALL`, `MATCH_CALL`;
`exe::Game::umpire_gaps`. `play.rs` queues the words at the point, the line call at the verdict, the others at
change ends / next serve / match over; `audio.rs` loads umpire 4 voice a (no umpire choice in the app yet), and a new
umpire word stops the last.
