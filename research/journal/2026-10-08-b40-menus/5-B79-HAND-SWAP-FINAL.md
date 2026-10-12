# B79: Select swaps the hand on the character select

- Select / − (pad), the camera key (C by default) on the keyboard: flips the driven player's hand toggle
  (`Player::swap`, kept across characters and costumes, only while not ready). 1P drives the computer players once
  every human is ready, so its press then flips the COM's.
- Card: "Right-handed" / "Left-handed" top right (gold when toggled), from TParam.csv's 利き腕 (`Text::hands`) or a
  mod's `hand`, negated by the toggle. Hint line under the top row: "Select / C: left or right hand". The 3D/PNG
  preview is mirrored (`UiTransform` scale x −1) while toggled.
- Match: `--swap-hands 1,0[,…]` by match slot (`play/hand_swap.rs`, `SwapHands` resource from `main.rs`). After the
  character's (or mod's) hand is set, `hand_swap::apply` keeps it as `Game::strong_hand` and flips
  `players[i].hand` and `reaches[i].hand` — so the model mirror and racket hand, forehand/backhand sides, contact
  search, serve walk and toss all follow, as the original's +0x12b4 follows the model flag (TParam left XOR toggle).
  The AI's strong side (+0x12dc, singles `stand_side` and the doubles AI's `strong`) stays TParam's.
- Test: `play::main_menu::tests::select_swaps_the_hand` — Ashley + Select, Will + Select → `--chars 0,11
  --swap-hands 1,1` → match setup from the ISO: hands [−1, +1], reaches the same, strong side [+1, −1].
- Shot: `context/b79/chars2.png`.

## Not verified / not 1:1
- The original's on-screen hand indicator wasn't looked at (front-end is ours to design, PLAN 2026-10-12); ours is a
  text label.
- Which hand the opponent's `opp_lefty` (doubles AI) reads for a toggled player (+0x12b4 or +0x12dc) wasn't checked;
  it uses the played hand. Untoggled matches are unchanged either way.
- The mirrored preview wasn't screenshotted toggled (no way to press Select in a `--shot` run); the match side is
  covered by the test.
- Online lobbies don't carry the toggle (the pick wire has no hand field): online players play the default hand.
- No PCSX2 lock-step of a toggled player's match; the toggle rule (flag = left XOR toggle, hand follows the flag,
  +0x12dc unchanged) is from the earlier research noted in `play.rs character_hand` and slot 3/4's Carol.
