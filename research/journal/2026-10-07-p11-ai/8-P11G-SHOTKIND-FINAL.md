# P11g — shot and serve kind choice (FINAL)

Port: `hst_sim::ai::Picks` / `AiParams::picks`; `hst_sim::aim::button`, `AiParams::lock_button`, `lock_stick`,
`serve_toss`, `serve_swing`, `serve_aim`; `hst_sim::serve::ai_search`; `hst_data::exe::Game::second_toss`.

Tests:
- `timing_errors_match_the_game` (`ai_s05.bin`): all 155 timing draws are followed by the picks, bit-exact
  (+0x244 quick serve, +0x245 dive, +0x24c body, +0x24d low, +0x5c..+0x5f serve/volley/return/high level);
  16 of them with the dive draw. `guesses_match_the_game` asserts the picks on each opponent hit too, and
  `reactions_match_the_game` draws them in place of the old 7 skipped draws.
- `aim::tests::plans_press_and_lock`: the plan → button table, the kind lock (button chain, flat/drop sticks bent
  back below `shot::stick_kind`'s lines), the toss and swing plans, a serve aim. Hand-checked against the decompile
  only: no fixture records a singles press or a serve choice.

## The original

- The picks (end of the per-hit draw, after the 4 timing errors and before the reaction): quick serve chance (row
  0x3c), dive (0x38), body (0x70), low (0x74), then serve level (3-way mix), volley level (4-way), return level and
  high level (3-way); `% 100 < edge` buckets. The dive is drawn only at a reset/serve (draw param 0) or on its own
  team's hit when the team's shot record (+0xf0) has this AI as hitter with kind 3 (its own dive); on an
  opponent's hit or a partner's it is kept. In the fixture a reset and a hit message without a shot look alike;
  the test lets either explain them.
- Plan byte → button (`button`): the chooser's plan (+0xb2) names a pad button, pressed at the aim's frame
  (frames + error < 9). The serve uses the same table for its toss (+0xb1) and swing plans.
- Singles kind lock (doubles has none), every frame with the frame's button word: △ → ✕ (`kind_lock[2]`), ✕ → ○
  (`[3]`, chaining after △), else ○ → ✕ (`[4]`); a non-zero button is kept on the AI (+0x2c). The frame before the
  contact: a kept ○ with the stick in the drop shot's 45° (−side·z/len ≥ 0.7071) has, `kind_lock[1]` % of the time,
  z bent to −side·|x|·0.96568877; a kept ✕ in the flat's 60° (side·z/len ≥ 0.5) by `kind_lock[0]` to
  side·|x|·0.55430907 — x through the pad byte (1..255) and z snapped to 1/127. So the "lock" keeps the button's
  kind: a ✕ stays topspin, a ○ stays a slice.
- Serve (state 3cee70): spot level 0 when the serve level is 0; toss strong on a first serve, on a second serve
  strong with the character's table chance (GAME data 0x41e410, 14 i32: 0,0,0,0,0,0,30,100,0,100,100,100,0,0),
  else weak; level 0 tosses underhand 10 % of the time. Contact pick (360010): the frame in the window closest to
  the ideal height while the ball falls, or still rises for a quick serve (never underhand). Swing at
  frames + serve error < 9: △ for underhand, else ✕ `serve_kind[1]` %, ○ otherwise. Aim the frame before the
  contact as a zone through `aim` (363740), by level/swing (`serve_aim`'s doc); a non-wide middle-depth stick
  never keeps z toward the side sign.
- Base level, key level and special return were already in P11f's choosers.

## In the app

`play.rs`: `ai_draw` draws the picks (`ai_picks`; `ai_dove` marks an own-team hit), `ai_press_kind` /
`ai_contact_stick` (the aim and button at the press, the singles lock at the contact), `ai_aim` passes the volley
and high levels, `bot_serve` → `ai_toss_kind` / `ai_serve_kind` (serve level spot, toss, swing, aim; the press
waits for `serve::ai_search`). `ai_second` per player from `second_toss`.

Ponytails: the serve aim is drawn at the swing press (the server stands still) rather than the frame before the
contact; the singles stick lock runs at the app's contact frame. The return and high levels only feed the
singles choosers' `Look` (receive isn't wired, P11f). Body and low picks are drawn but still unread by the app's
contact search (P11h / the AI's own search).
