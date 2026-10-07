# N3c3: swing whoosh (FINAL)

## Path
- Effects manager event 2 (swing start): low nibble shot kind into fx+0xd0, high nibble player; it copies the player's branch (+0x3ec1) and starts that player's swing-trail object (`0x3441c0(obj, 1)`).
- Trail start: branch 0 (serve) with faults `0x316608` == 0 → countdown +0xa4 = 4, played by the trail update (`0x343610`) when it reaches 0. Branch 4 (smash) with kind != 3 (lob) → plays at once. Every other swing plays only when trail +0xa0 == 1 and a scene counter check holds. +0xa0 is a copy of gm+0x344 (taken on event 0xe), and gm+0x344 != 0 cancels the play, so that path needs gm+0x344 to change between the two. It is 0 for all 3600 frames of hits_s05 (gm+0x345 toggles, +0x344 never), so in normal play it never fires.
- Play: slot 0 program 4 key 0, volume 0x80, positional at `0x348fe0(player) + 0x30`. That is player+0x3d70, the translation of the player's model matrix (+0x3d40, handed to the model by `0x14c5b0`), not a hand bone (the plan's guess). So the whoosh plays at the player's position.

## Proof
`hits_s05.bin` has 6 whoosh key-ons: 4 first serves and 2 non-lob smashes. Each lands 3 to 4 frames after the server's branch flips to 0, and at the smash's swing frame. No ground stroke, volley or lob gets one.
Test `swing_sounds_match_the_game` (crates/hst-sim/tests/sound.rs), using faults from the recorded rally block (+0x18):
- `sound::swing_sound` predicts a whoosh before exactly those hits, and the counts are equal.
- A serve's whoosh L/R matches the recorded play triple bit-exactly.
- The smash whooshes come out at bearing about 341° against a triple of 343°, because the triple belongs to a later play. Player positions are not recorded, so their bearings are not checked.

## App
`play.rs` `whoosh` runs when a swing starts (serve swing locked in `serve_turn`, stroke contact found in `advance_stroke`). `play_sounds` counts the waits down and plays `sound::SWING` at the player's position. In a `--play` run the first serves whoosh, a second serve doesn't, and neither do strokes.
