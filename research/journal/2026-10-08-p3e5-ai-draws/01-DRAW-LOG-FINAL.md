# 01 — draw log, new point, hit message

## Capture

`timeout 600 tools/pcsx2.sh python3 research/p3e5_draw_log.py 5 1250 context/p3e5/draws_s05.bin`: a hook on the MT
draw (0x19f5c0) logs (vsync, ra, the percent roll's caller from sp+0x10, its percent in s0, gm+0x58) for every draw
on the AI generator (0x427130). 1186 draws over 1265 frames from vsync 7566. Its per-point totals equal rng_s05's.
gm+0x58 (the phase frame counter) groups the draws by tick; the vsync stamps don't (samples are taken mid-tick).

## New point

rng_s05's 72/21 at 7566 (68/21 at 8656) is a tear. The event is 93 draws (89) over three ticks:

1. The new-point message, AI 0..3 in turn: the net pick at even odds (match start, bot partner), 8 timing draws,
   the picks (quick serve, dive, body, low, 4 levels), the hit count. 18 each (17 later).
2. The state entries in AI order: the server's timing, picks and count (17); the receiver none; each rallying
   partner's return-to-centre roll (`chance(ai+0x18)`, `Return::new`) in 0x3d0110, plus a net repick and count when
   its count is below 1 (not at a new point).
3. The serve routine (0x3cee70): spot (level 0 only), wait.

`ai_draws_like_the_game` (tests/rng.rs) draws this recipe from the 7565 and 8655 generators and lands on the 7567
and 8657 ones exactly. The counts don't depend on the rows, except the spot (serve level 0 in both).

App: `ai_update` returns after the reset (the message tick, `bot` does nothing else), draws the serve entry on
`start`, and `bot` waits that tick too; `bot_serve`'s first call draws only the spot and wait.

## Hit message

Per AI in order (rally state, re-entering substate 0 unless in 3 or on the serve): return-to-centre roll (3d0180),
net repick and count when used up (3d01dc, 3d01ec), pace chances (3d27c0, 3d2548), timing and picks (15), reaction
(3d2de0, 3d2ec4, 3d2da8), guess (3d3004, 3d3018), then the formation repick's return roll (3d59f0 in 0x3d51e0).

The app drew the return roll in `doubles_ai::step` after the whole message, and the net repick on every hit while
rallying. Now `doubles_ai::heard_hit` draws the return roll in the message and keeps it for `step`'s re-entry, and
`ai_heard_shot` repicks only on that re-entry. The 3d51e0 repick still runs in `step` (needs the routine's Body and
World in the message: P11k6). A per-hit count test needs the full rally state replayed (P11k6's frame-exact replay).
