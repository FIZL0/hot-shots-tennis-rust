# The pick

## What the inputs are

- 0x422fc8+4·i: the controller words. A pad index (< 0x20) for a human; 0x21 for a computer player. The player
  constructor copies it to +0x13f0.
- 0x422fd8+4·i: the AI setup words, passed to the player constructor (0x345510) and on to 0x35edb0 → 0x35cf70. Byte 2
  becomes AI +0x11, which the app already has as `ai::Choice::strategy`. The menus leave byte 2 at 0 (slot 5 reads
  0, 2, 1, 5; slot 3 reads 0x30, 4, 3, 0xb).
- AI objects: every computer player gets one; a human gets one only in doubles beside a computer partner. Two humans
  on a team have none, so the pick never runs for them and they keep 0.
- 0x423040 is set at the match start and cleared after the first point (slot 4 reads 0), so the pick runs only at the
  match's first point. Later points skip it and keep the byte.
- Placement runs on the phase-entry messages (6, 0xc, 0xe). 3449f0 always draws once on entry; +0x13f5 set skips the
  formation pick.
- The broadcast (0x18b310 → 0x18b390) walks priority levels 0..5. At each level it calls the handler when the
  object's +4 byte matches, then recurses into the children, which are kept in insertion order. The players are made
  in order, so player i takes the reseed's draw i (the generator's index reads 4 afterwards).

## Recordings

`p3d2_formation.py <human> <points>` loads slot 5 and sets up these pokes:

- player `human`'s controller word set to 0;
- row byte 2 set to 1 for player 0 and 2 for player 2;
- AI +0x11 set to 1, 2, 3, 1;
- 0x423040 set back to 1 whenever the game clears it.

At each new point it logs the reseed's first 4 MT words and every +0x13f4, then marks +0x13f4 with 0x77, so a pick
that didn't run would show. The first runs were real time with a 0.3 s settle and 32-bit read-modify-write pokes. The
last run is lock-step with PINE's 8-bit write.

| Run | Who is human | Points | Agrees with `Team::pick` in player order? |
| --- | --- | --- | --- |
| h-1 | nobody | 13 | yes, with 4 staggered zeros |
| h0 | player 0 | 6 | yes: everyone 2 (partner's row, copied) |
| h2 | player 2 | 6 | no: 1 point where player 1's result needs draw 0 or 2 |
| h2b | player 2 | 18 | no: 1 point where player 1's result needs draw 0, 2 or 3 |
| h2c, lock-step | player 2 | 30 | no: 2 torn reads (index 1, 3) and 1 point where player 1's result needs draw 0 or 3 |

With player 2 human, about one point in ten has player 1 taking draw 0 instead of draw 1. Every one of those points
still ends with the generator's index at 4. Player 2 was only poked human mid-match, and a real setup puts the pad-1
human on player 0, so this order shift is left as P3d2a.

## Port

- `hst_sim::position::Team::pick`: the four cases, plus the `(draw>>16 & 0x7fff) % 100 < 20` stagger.
- `play.rs`: `Player::formation`. `placement_draws` now returns the draws. The setup records `Game::humans` and calls
  `doubles_ai::formations` after its placement draws. `reset_positions` keeps the formation across points.
  Placement (`serve_placement`), `ai_wait`, the pair aim and the AI body read `p.formation` instead of the old
  `human_mate ? row : 0`.
- `doubles_ai::formations`: players 0 and 1 pick (setup byte 0, as the menus leave it); 2 and 3 copy their partner;
  everyone is placed again for the serve.
- Test `formation_pick_matches_the_game` (`crates/hst-sim/tests/position.rs`): 76 picks from h-1 and h0, each run's
  first line left out.
