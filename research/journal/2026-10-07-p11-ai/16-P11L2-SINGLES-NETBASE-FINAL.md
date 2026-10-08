# P11l2: the singles NET and BASE rally routines (final)

## Recording

`HST_SINGLES=1 tools/pcsx2.sh python3 tools/record_ai_rally.py 8 6000 context/fixtures/ai_rally_singles_net.bin 1 1,2,3`
(slot 8 = `bots_singles`). The singles regions now also take the opponent's hand byte
(opp+0x54 → 0 → 0 → +0x135, at record 0x521), which the BASE chooser reads.

Test `singles_net_and_base_match_the_game`: NET 2253 calls (substates 1007/603/1/642), BASE 4605
(1738/1443/211/1213), all bit-exact (AI object, stick, button, draws, path copy, seen, lost log).
The singles receive tests still pass on both older fixtures.

## Singles AI fields that differ from doubles

| Field | Doubles | Singles |
|---|---|---|
| walk back wait/going/there | 0x260/0x264/0x265 | 0x250/0x254/0x255 |
| net pick left/rate/net | 0x26c/0x270/0x274 | 0x258/0x25c/0x260 |
| dash | — | 0x256 |
| short-ball flag (BASE chooser) | — | 0x257 |

The dispatcher picks NET/BASE from the singles net byte (0x260).

## Set-sub 0x3ca3d0

- **Sub 0:** fresh, next = -1, chase; the walk back is redrawn (rate 0x18); then the net repick (`Mind::rally`).
- **Sub 2:** lead by kind (4 → 0, 3 smash, 2 volley, 0/1 stroke).
- **Subs 1 and 3:** nothing. There are no shot records in singles.

## NET 0x3ca4e0 / BASE 0x3cb130 vs doubles

- **Searches:**
  - smash, then volley (NET only);
  - BASE drops a smash found below its ideal height (reach.smash[1]).
- **Dash** (0x3cbb10). The dash ends once the ball is on its side and deeper than the player. While dashing:
  - BASE tries the volley (all entries) first;
  - then tiers from the next entry (low → from tier 3).
- **Fresh ball.** Not landing short:
  - BASE: the reach search, kind 1;
  - then tiers with min bounces 1, then 0.
- **No find:**
  - landing short → landing(0.3) / after bounce / entry 11 / last;
  - else the dive (not phase 4, not `no_dive`);
  - else the chase walk.
- **Walk back.** Inline, dz² first: `madd(dz², dx, dx) <= r²`. So it is not `Return::step`.
- **Choosers:**
  - NET: 0x3cc700 (`AiParams::receive_aim`, misnamed: it is the NET chooser);
  - BASE: 0x3cce80 (`rally_aim`, with the short-ball flag).
  The Look gains opp_kind (ai+0x124), opp_lefty, the mark (ai+0x130) and heights incl. under_min.
- **Sub 2, NET.** After a volley or smash, dash to the shot's x, clamped to ±2.743333 (0x402f92c5), at the net reach.
  - Plan 10 stands a third of the way back.
  - BASE only sets dash after a smash.
- **Sub 3, NET**, stroke over and not dashing:
  - Zones (court 4.115): the target, the opponent (blurred, draws) and its own.
  - Middle shot, or a target in the opponent's zone: stay at its depth (no deeper than 10).
  - Otherwise: ⅔ back from the net reach, no deeper than the opponent; dash if either is in zone depth < 2.

## Branch coverage of the recording

Reached:
- smash (NET), NET volley, BASE dash volley, dash tiers, dash off;
- reach search, fresh tiers (BASE), dive, landing short, there;
- plan-10 dash spot, after-hit dash.

Not reached (→ P11l5):
- NET fresh-ball tiers;
- the after-hit middle spot and its opponent-depth clamp;
- the walk back's 60-frame redraw;
- the line-margin hold;
- the chase walk's arrival;
- BASE smash (taken or rejected by height).
