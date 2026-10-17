# Team reaction faces (2026-10-17, FINAL)

User: Carol and others show no face animation in post-point reactions. Audit of every in-match face:

- Singles reactions (0x2b..0x2f), strokes, runs, stances: every character's `.MOR` tracks bind and play (checked all
  14 characters' reaction tracks against their morph targets; only pc07/pc13's trailing-space `sorrow_mouth    ` and
  their missing `blink_eye` stay unbound, as the game drops them too, N1d4).
- Gap: the doubles team reactions (motions 0x30..0x34, `re_pc00_co01_f`..`co05`) had no face at all. The N1d4 notes
  left "co faces from PCCG0 MtGrl, the +0x1404 team-reaction offset" undone.

## Found (decompile)
- The player loader adds the first 0x30 motions' faces, then (doubles only, count 5) the team-reaction loader's ten
  per character: slots 0..4 `MtGrl/pcNN/re_pcNN_<co>.MOR/.UVA`, slots 5..9 the `_2.MOR/.UVA` takes.
- The motion setter: motion > 0x2f → face = motion, + count (5) unless `player index & 1` equals the point's
  winning team (the global the reaction handler sets with the gu/di choice). So winners show the base face,
  losers the `_2` take.

## Built
- `character.rs`: faces 0x30+k / 0x35+k loaded from PCCG0 (own motions' faces only for ids < 0x30, as the game).
- `play.rs` `motions`: after a team reaction's `set`, the losing team's face += 5.
- Tests: `uv_tests::team_reactions_have_faces` (which sets each character has, both takes, co02_f's UVAs);
  `serve_check` B270 test checks each player's face (mutation-checked: without the +5 it fails on a loser's co03).
- pc08's `co02_f_2.UVA` names no skeleton node, so it never plays (game binding).
