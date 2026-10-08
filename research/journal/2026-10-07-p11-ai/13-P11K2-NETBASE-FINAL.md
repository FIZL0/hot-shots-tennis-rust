# P11k2: the doubles NET and BASE rally routines (final)

NET (0x3d02c0, tag 2) and BASE (0x3d1320, tag 3) are one body with a style switch. They are ported as
`rally::Rally::rally(net, …)`. The test `net_and_base_match_the_game` (`crates/hst-sim/tests/ai_rally.rs`) replays
every tag-2/3 call. It compares everything the receive test compares, plus the four shot records.

| Fixture | Frames | Calls NET (0/1/2/3) | Calls BASE (0/1/2/3) |
|---|---|---|---|
| `ai_net_s05.bin` (`record_ai_rally.py 5 3000 … 1 2,3`) | 3040 | 3975 (3341/248/23/363) | 5232 (4008/473/235/516) |

Slot 5, bot-only doubles. Every call matches bit for bit.

## Recorder fix

The `("abs", GM_PTR, off)` regions loaded `*(GM_PTR + off)`, but gm itself is `*GM_PTR`. This zeroed the ball (0x540),
the hit counts (0x550), the gravity/drag (0x710) and the path object (0x560) in every earlier fixture. The regions now
dereference gm first.

The receive was re-checked on a fresh recording with real values (`ai_rally_s05_fix.bin`, 3027 frames, 916 calls) and
is still bit-exact. The older receive fixtures still pass too.

## Shot records

The records sit at the path object +0x70 + slot·0x30 (recorded at 0x580):
- +0 stamp, +4 n, +8 kind, +0x10 pos, +0x20 vec.
- Write (35e510): copy the fields, then stamp = frame.
- Read (35e560): n += stamp when n ≥ 0.

So n is either the contact frame or a negative marker:
- -2: no shot;
- -3: marked after the stroke.

## The routine

- **Substate 0.**
  - Return while held: the point's first shot, fewer than 2 shots, or the hold after a net-zone end (+0xbc).
  - Wait countdown.
  - Its own team's hit (+0xd6): every 30 calls a formation repick on the partner's position. Me when it hit, else
    Other. Then the walk back.
  - Follow (+0xd4), when the partner's record is set and it isn't fresh, or it plays front with the ball on its half.
    It repicks on the record's target and walks back.
  - The follow ends when the record is stale or the ball passes the partner (frame + 8, partner z + 2). It then holds
    on the side away from the ball (x = ∓2.7425) unless it is the middle pick.
  - Otherwise the path copy and the searches:
    - NET: smash, volley, then the tiers when the ball is forward (3d23f0 / 3d5dd0), else fresh: tiers min 1 then 0.
    - BASE: smash, volley(all) when forward, else fresh: reach(1), then the same tiers with all = true.
  - A find sets substate 1 and runs it in the same call.
  - No find:
    - Short landing: landing / after-bounce / entry 11 / last; this walk keeps +0x99.
    - Else 362090 (the entry after the last 1-bounce entry, when `t` is on the other half): a set partner record →
      repick Other and follow.
    - Else 361f50 (0 bounces), when the partner is nearer → middle repick.
    - Then dive (kind 4, pair aim, button, substate 3).
    - Or, while chasing (+0x99), after-bounce / entry 11 / last, walking there; arrival clears the chase.
    - The search restarts from n - 1 (+0xd0).
- **Substate 1.** Give way (3d5bb0) on the two records: repick on the partner's target, follow, voice, back to
  substate 0 (kept). Otherwise as the receive's substate 1 (ready check, swing frame).
- **Substate 2.** A stale own record is cleared to -2. At swing + lead < 9: path mode 2 with gap ≥ line margin → held
  and substate 0, else pair aim + button and substate 3.
- **Substate 3.** Stick at swing 1. The mark puts -3. The stroke over → substate 0. The wait counts down while the
  opponents have the ball.

## Quirks

- 361f50 and 362090 test the sign of an incoming stack quad `t` that nothing writes when no pick ran. The port keeps it
  across calls (`Rally::stash`). It matched on this recording; the game's is whatever its stack held (P11k5).
- The voice line on giving way draws one extra roll when it speaks (partner +0x3fa5 < 2, 25%). The sound isn't played
  (P11n).
- 3d5bb0 compares the records' frames with a wrapping |t − m| < 30.

## Coverage (branch counters, NET + BASE)

Reached:
- own-team repick 78;
- follow start 12;
- follow end/hold 3/176;
- smash 3, volley 7, forward tiers 3, fresh reach 10, fresh tiers 11;
- 362090 find 11;
- middle repick 3;
- dive 2;
- chase walks 269;
- give way 12 (voice roll 3);
- ready check 743;
- substate-2 aim 22;
- mark put 1.

Never reached:
- the short-landing not-found path;
- the tail's repick from the partner's record;
- the substate-2 net-zone hold;
- the substate-3 wait countdown;
- the give-way defer;
- every human-partner branch.

These are ported from the asm and tracked as P11k5.
