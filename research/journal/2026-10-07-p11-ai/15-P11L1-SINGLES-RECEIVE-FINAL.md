# P11l1: the singles receive routine (final)

## Recording

`HST_SINGLES=1 tools/record_ai_rally.py <slot> <frames> <out> [every] [tags] [slow]` hooks the singles routines:
- receive 0x3c9a40 (tag 1), NET 0x3ca4e0 (tag 2), BASE 0x3cb130 (tag 3).

The record layout is the doubles one, with these changes:
- the opponent (*(ai+0xd4)) +0x3d70 at 0x500;
- the player's +0x3e90 (its shot's target) at 0x4f0;
- no partner regions.

The save state is `context/recordings/bots_singles.p2m2_SaveState.p2s`, copied to slot 8.

| Fixture | Frames | Tags | Receive calls | Substates 0/1/2/3 |
|---|---|---|---|---|
| `ai_rally_singles.bin` | 6000 | 1-3 (NET 2253, BASE 4605) | 2015 | 1500/191/80/244 |
| `ai_rally_singles_long.bin` | 42000 | 1 | 10276 | 7391/1158/358/1369 |

Test: `singles_receive_matches_the_game` (ai_rally.rs). All 12291 calls are bit-exact.

## The original vs the doubles receive (asm diff)

The routine is instruction for instruction the doubles one (0x3cf7b0), except for four things.

1. **Field offsets.** The fields 0x234..0x25f sit 0x10 lower:
   - leads 0x224/0x228/0x22c, dive 0x235, wait 0x238;
   - body/low/push/guess 0x23c..0x23f, from 0x240/0x248.
   The set-state routine 0x3c9980 is the 0x3cf6f0 twin with the same shift.
2. **The return aim.** It calls 0x3cdd00 in place of 0x3d4420.
3. **The stroke-over spot.** When the stroke is over and the dash flag ai+0x256 is set:
   - stand x (ai+0x70) = player+0x3e90 (the shot's target x, unclamped);
   - stand z (ai+0x78) = -ai+0x14 (net reach) * side.
4. **Return value.** It is the same in both.

## The singles return aim 0x3cdd00 (`Rally::singles_aim`)

**Inputs.** `zone_in` on a 4.115 court (from ai+0x38). The opponent's zone is blurred, so it costs MT draws.
- The opponent comes from ai+0xd4 via 0x348fe0 (player+0x3d40 matrix, translation row = +0x3d70).
- deuce = court word 0x423050 == 0.

**Order of picks.**
1. **Out of court:** |x| > 5.485. With p = min((|x| − 5.485)·100, 50), roll(p/2.5):
   - coin 50: plan 10, lane = own lane ≠ 0 ? 0 : 2, depth 2;
   - else plan 11, lane = own lane == 0 ? 2 : 0, depth 0;
   - then roll 30 → lane = draw 3.
   - The lanes are flipped against the doubles aim for plan 11, and the 30% re-pick applies to both plans.
2. **Special return:** roll row+0xa4 (`special_return_rate`). Depth 2; lane away from the opponent's lane (the middle lane: coin 50 → 0, else 2); plan 10.
3. **By ai+0x5e:** the `return_level` pick drawn in 0x3cbbe0 from row+0xd4. All branches set plan 7 unless noted.
   - **2, roll 60 passes:** lane deuce ? 0 : 2, depth 2.
     - At level 3 with |opp z| < 7.4: roll 10/20/5 (NET/ALL/other) → plan 10.
     - Dash: NET roll 20, ALL roll 10.
   - **2, roll 60 fails:**
     - opponent lane 0 or 2: lane 1, depth 0;
     - otherwise: lane deuce ? 2 : 0, depth = roll 90.
     - If opp lane 1, opp depth 2 and depth 0, then the same dash roll.
   - **1:** lane = coin 50 ? 0 : 2, depth 2. Then NET roll 30 or ALL roll 15 → plan 8 and dash.
   - **0:** lane from the blurred zone of ai+0x130 (the opponent's last shot record's spot).
     - Depth 2 on NET roll 50 / BASE 90 / ALL 70, else 1.
     - Side lane: roll 10 → middle. Middle lane: roll 50 → deuce ? 2 : 0.
     - Depth 1 zeroes stick z (ai+0xa8).
   - Then `aim` (0x363740) writes the stick at ai+0xa0.

## Coverage

Aim calls, as (return level, plan, dash) → count:
- (1,7,0) ×4, (1,8,1) ×1;
- (2,7,0) ×18, one of them out of court with the roll failed;
- (2,7,1) ×1.

The stroke-over dash spot ran 4 times.

Never reached: return level 0, the out-of-court plans, the special return and the level-3 plan-10 upgrade. These are tracked as P11l4.
