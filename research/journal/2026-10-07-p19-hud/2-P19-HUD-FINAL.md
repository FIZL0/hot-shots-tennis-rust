# P19 — head markers, "Score to win" line, old score text dropped (FINAL; menus split to P19a)

## Head markers (`play/markers.rs`)
- Owner: the balloon/marker manager (find it in RAM by its vtable word; `research/marker_rec.py <slot> <vsyncs>`
  logs its show flag +0x725 against gm phase and the HUD panel). Messages: 0x11 on (more than one player), 0x10 off
  when the server's toss motion starts, 6/0x12 off, the point reset on again.
- Recorded (slot 5): on at serve-phase tick 1 together with the panel slide-in, off at the toss (ticks 91, 125).
- Draw: `azuma/panel/i_playerinfo.tm2` in `AZUMA/C_EFF/EFFCT.XB0` (256×32, cells 1P 2P 3P 4P COM, u step 0.156),
  tinted with the panel's pill colours (same tables at runtime), alpha 128, no fade. Bottom edge (pointer tip) at
  the player's x/z and height reach-base (TParam col 55) + 0.65; half-width 0.3·max(1, 0.15·z·t)·min(1, 0.2·z·t)
  (z view depth, t tan of the half-angle; the game's fov constant is 20° full), height 0.84 of the width.
- Check: `context/shots/p19/mk_0.6.png` vs `context/shots/b5/orig_100.png`: "COM" ≈3.96 % of screen width in both.
  Singles (`singles_1.5.png`) and human (orig `orig_poke_*.png`: 1P marker) seen.

## "Score to win" line (`play/panel.rs` game_info + layout)
- Panel draw: once slid in (cnt > 4) and not deuce, for the first team with a game point: row of `i_gameinfo_00`
  (256×24 rows): game (server's team) / service break (receiver's) when winning the game doesn't win the set
  (games < set games − (leading ? 1 : 0), never in a tiebreak); else set when sets < sets-to-win − 1; else match, or
  "Score or you lose the match!" addressed to the other team when the match-point team has no human and the other
  has one. Drawn native size at x 0 / 384 on the addressed team's side, y 300 when its panel is at the bottom, 124 at
  the top. The addressed team's "A/B Team" banner is not drawn while the line is up.
- Blink: a counter from when the panel is in: alpha 0 for 15 ticks, 128 for 15 (same clock as the ring); during the
  rally fade it fades with the rest (128·c/5).
- Verified on the original by poking the score in slot 3 (P1 serving, waiting): `research/gameinfo_poke.py 3 <out>
  <p0,p1> [g0,g1] [s0,s1]` → `orig_poke_*` (this game, left, y≈300), `orig_break_*` (service break, right, y≈124),
  `orig_lose_*` (lose the match, left, A Team banner gone). F8 screenshots lag ~1 s, so a moving serve (slot 5) is no
  good for this (`research/gameinfo_shot.py` attempt).
- Test: `play::panel::tests::score_to_win` (row choice, position, blink, banner hidden).

## Other
- Post-point score show (HUD mode 1) is P12b1's `play/popups.rs`; nothing new here.
- `play.rs` hud: the text score line and player list are gone; only messages, controllers, camera mode and the
  controls help stay.
- The port's autoplay doubles stayed at 0-0 for 60 s in `--shot-at 60` (a fault, then serving again) — not looked
  into; a port screenshot of the line at a real game point is still to take (no score override flag).
- In-match menus → P19a (pause sprites `inpane_pause00`–`03`, `i_dialog`, `KeyAssign_inpane3`).
