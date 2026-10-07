# P11e1: doubles formation and the walk back (done)

P11e was too big to verify in one go from slot 5 (an all-bot doubles match with net-dash rate 0 everywhere), so it is
split: P11e1 (this, done), P11e2 singles return to centre + net dash, P11e3 guessing (ヤマ張り), P11e4 strong-side
run-round.

Port: `hst_sim::position` (Formation::start / repick / hold, Return::new / step, Team::lean). Test:
`crates/hst-sim/tests/position.rs` against `context/fixtures/ai_pos_s05.bin` (tools/record_ai_pos.py 5 3000): every
change of a bot's lane / front flag / spot over 3000 frames is reproduced exactly by one call (8 point starts, 38
re-picks, 3 holds), and all 34 "arrived" transitions happen just inside the centre radius.

## The original (doubles AI object = player+0x80)

- Centre numbers (35ef00, on reset and on several messages): +0x38 is the **singles** flag (0 in doubles: the zone
  classifiers use the 5.485 doubles width when it is 0). In doubles beside a human partner: rate/radius = row
  0x88/0x8c, reaction 0x48/0x50, mode 1; otherwise rate/radius = row 0x80/0x84 (the "singles" columns, also for an
  all-bot doubles team), reaction 0x44/0x4c, mode 2 if self is human else 0. Rate at AI+0x18, radius at +0x1c.
- Formation state: +0x266 lane (1: the half on the side sign's +x, 2: the other), +0x267 front, +0x268 (= front in
  formation 0, 1 in formation 1, 0 in formation 2), spot at +0xc0/+0xc8. Formation byte = player+0x13f4 (0 for all
  four slot-5 bots).
- Point start (3ce4c0 → 3d4fa0): server and receiver are back, their partners front. Lane 1 when (starter) ≠ (ad
  court). Spot x = side·2.7425·(lane 1 ? 1 : −1); z = side·(−11 back / −3.2 front) in formation 0; the routine tests
  the byte for 0 twice, so formations 1 and 2 both get −11 / −7.9 there (kept as is).
- +0xe0 "lean" (3ce4c0): formation byte 0 and both bots: 1 for a NET bot beside a BASE partner, 0 for BASE beside
  NET, else −1. It biases the front test by ±0.75 m and pulls a back NET-lean bot to 10 m (9 after its own hit).
- Re-pick (3d51e0), callers: the hit message (own team hit: the hitter's shot record), the waiting state every 30
  frames (self last hitter → own position vs the partner's; else the partner's position) and the partner-shot
  paths (the partner's shot record from the gm+0xa4 table, entry +0x20 = position). Front = lean_a + |own z| ≤
  lean_b + |other z|. Lane: own hit at the net → the half it stands in; own hit at the back → moves off a wide third
  (3-lane split at 5.485/3); another's shot at the back → the half the shot comes across; at the net → follows a
  wide third. Depths: formation 0 back −(11 − lean), front −(3.2 + 2) after another's shot unless its own team hit a
  ball deeper than 3.2 on this side, −3.2 after its own; a shot from outside the singles court (|x| > 4.115 or
  |z| > 11.885; 3636a0 with arg 1) holds −6.2 (x = 0 when |x| > 5.985). Formation 1: back −4.9, front +0.5 instead
  of +2. Formation 2: back −11, front −7.9. Cue −1: spot (0, −6.4·side), lane/front kept. A changed spot clears
  +0x260, rolls +0x264 = roll(rate), clears +0x265.
- Hold (3d5160): spot = own position with x = −sign(ball x)·2.7425.
- Walk back (3d5a30): arrived (+0x265) → stay; roll failed (+0x264 = 0) → count +0x260 to 60 then re-roll; else walk
  while (spot − pos)² > radius², then set arrived.

## In the app

`play.rs` `ai_wait` (doubles bots, when the ball isn't theirs): places at the point start, re-picks on each team hit
and every 30 frames after, walks to the spot through `Return`. Ponytails: it uses the hitter's position instead of
the shot record and skips the hold. `reset_positions` now keeps `ai` and `ai_hit` across points (they were reset to
defaults every point, so after the first point the bots had a zero row).

## Open (the split)

- P11e2: singles return to centre — the singles wait state inlines the walk (+0x250/+0x254/+0x255, target +0xc0/200
  from the after-hit state's 3625b0 zones) — and net dash (row 0x40 rolled in the serve routines 3c8f10 / 3cee70;
  after a shot of kind 2/3 the spot is (player+0x3e90 clamped, −reach·side)). Needs a singles bot recording (slot
  3/4 with vpad driving P1) or a row with a net-dash rate.
- P11e3: guess — drawn at the tail of the timing draw (0x24f = roll(0x10c) ? roll(50) ? 1 : 2), move frames 0x110,
  stuck 0x114 when wrong; used in the receive state 3cf7b0. Its MT position follows the P11c/P11g/P11h draws.
- P11e4: strong-side run-round — roll(row 0x78) with extend row 0x7c in the contact searches 360150/360910.
