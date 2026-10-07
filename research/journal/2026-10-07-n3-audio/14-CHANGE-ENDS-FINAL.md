# N3g — change-ends music (FINAL)

Result: entering the change-ends phase plays jingle program 0 key 0 on slot 8 (`jig_00`, volume 0x80). This is the
change-ends tune. The BGM fades down for it the same way it does for the game jingle.

## Original
- The match phase machine sends message 0xc to every object on the tick it enters phase 1 (change ends). This is
  the same entry `match_s05_post_point` already checks tick-exact against `CHANGE_ENDS`.
- The music director's message 0xc handler:
  - plays slot 8, program 0, key 0, volume 0x80 through the same jingle wrapper as message 0x19, which sets the fade
    mode to 2 (fade down) with floor 0;
  - leaves the hold flag alone, so after a game it stays set from the game jingle until message 0xe, the next serve;
  - re-reads the weather for the rain ambience (not ported here).
- The per-frame update fades the BGM up only once the jingle has stopped and the hold is clear. In a tiebreak change
  of ends there is no game jingle hold, so the BGM returns as soon as the tune ends.
- A new jingle does not stop the one playing. Message 0xe stops the jingle only under the close-up flag (gm+0x344),
  so normally the tune plays on into the next serve.

## Proof
`change_ends_tune_matches_the_game` (hits_s04): the game jingle keys on at frame 1310 and the change-ends tune at
1458, 148 frames later, after key 1's last note at 1430. Every jingle key-on before 1458 is a tone of key 1 and every
one after is a tone of key 0.

## App
`play.rs` sets the jingle to key 0 when the post-point phase asks for change ends. `Bgm::step` fades for key 0 and
holds only for keys 1 and 2. Jingles are always on, so the tune plays without `--music`.
