# P11e2: singles return to centre and net dash (done)

Port: `hst_sim::position::{zone, Single, Court}` (Single::start / after_hit / go_in / passed / step; the walk reuses `Return`). App: `play.rs` `ai_wait_singles` (called where `ai_wait` gives None), and `bot()` notes the swing branch from `contact` into `Player::ai_branch`. Test: `crates/hst-sim/tests/singles.rs` against `context/fixtures/ai_pos_singles.bin` (5400 frames). Over those frames: 10 point starts match the style's centre; 13 net-player after-hits reproduce the spot and the dash flag and spot exactly (the opponent's blurred zone is tried with both MT extremes); all 5 dash switch-ons come from a volley or smash contact or from the after-hit; all 20 arrivals happen just inside the radius.

## Recording
- `tools/record_ai_pos.py` now writes a header count `n | 0x100` (n = the nonzero gm+0xa8 players, 2 in singles). After the per-player blocks it adds, per player, extras: pl+0x3e90 (the shot's target), +0x3ec0 and +0x3f90 (0x10 each). The old ai_pos_s05.bin (count 4, no extras) still reads in tests/position.rs.
- Bot-only singles: there is no singles save state. Starting from slot 1 (menu): cross x2 -> Main Menu -> circle (Fun Time Tennis) -> left + circle (Singles) -> 1P COM (down x5, circle), 2P COM (down x4, circle) -> JJ (char 5, BASE, row 5) for 1P, Jun (char 2, NET, row 2) for 2P -> Confirm -> circle. Circle confirms and cross backs out. Scratch slot 9 = the first serve (slot 8 = loading). Then `tools/record_ai_pos.py 9 5400`. Rows 2/5 at level 3: net-dash rate 0, centre rate 100, radius 0.7/1.0, reach 3.2. Ends change mid-recording, so the test takes the side from the position.

## The original (singles AI object, vtable 0x1d20e0)
- Init (each point): centre (0, side*-10) for NET style, (0, side*-11) otherwise; dash flag +0x256 cleared. ALL style: coin +0x260 = roll(+0x25c = 50), re-rolled on entering wait whenever the counter +0x258 < 1 (set to rand%3+2, counted down in shot choice).
- Rally dispatch: NET -> the net function, BASE -> the baseline function, ALL -> by coin.
- Entering wait (state 0): +0x250 = 0, +0x254 = roll(rate +0x18), +0x255 = 0.
- Wait walk (ball not coming): with the dash on, go to +0x70/+0x78 (NET) or (0, -reach*side) (BASE). Else when arrived, stay. Else when the roll passed: arrive once dx^2+dz^2 <= r^2, else walk to the centre. Else count to 60 and re-roll. Same shape as the doubles `Return`.
- NET after-hit (end of state 3, when player+0x3f95 == 0 and no dash): zone split 3625b0 of the shot target (exact), the opponent (blurred, MT rolls) and itself (exact). x = 0. If the target lane is 1, or target zone == opponent zone: z = own z, capped at 10 deep. Else z = side*-((11.885-reach)*2/3 + reach), but no deeper than the opponent's |z|; dash on if its own depth band or the opponent's is < 2. With the dash on: +0x70 = target x clamped to +-2.743333 (DAT_004178e0/e8 = DAT_004178f0/f8), +0x78 = side*-((11.885-reach)/3 + reach) if shot choice 0xb2 == 10, else -reach*side. Then enter wait.
- Zone split: half-width 4.115 (5.485 when AI+0x38 = 0), depth past 1.5. Exact: lane 1 if |x| <= w/3, else 2/0 by x*(z<0 ? 1 : -1); bands at 3.4616668 and 6.9233336. Blurred: lane edge band +-((2w/3)/4) around w/3 and depth bands 2.59625-4.3270836 and 6.057917-7.78875 (divisor 1.7308334), each decided by u*2.3283064e-10 < fraction.
- Dash on: a NET contact with 0x9a = 2 or 3 (volley/smash; 0x9a picks the timing error: 0/1 stroke +0x224, 2 volley +0x228, 3 smash +0x22c, 4 none); a BASE contact with 0x9a = 3; message 0x15 when the opponent's shot kind (+0x124) is 3 (smash) and the row style is NET or ALL -> (0, -reach*side); the serve (3c8f10 case 5): the wide-serve case with NET (or ALL plus roll(80)) -> x = u*1.3716666 (+1.3716666 unless 0xb2 == 3) * (ad ? 1 : -1) * side, z = -reach*side; then always roll(row 0x40) -> dash on.
- Dash off (3cbb10, in the incoming-ball path): the ball (+0x38 of the ball object) is on its own side and deeper than it.

## Open / simplified in the app (ponytail)
- Wide-serve dash (needs P11's serve aim), the serve-dash x (the middle here), and shot choice 10's deeper dash spot (needs P11's shot choice) are not ported.
- The ALL coin is drawn once per point.
- The dash-off test runs on the wait frames, and the after-hit runs on the first frame after the swing.
- The app's AI is always level 3 (reach 3.2).
