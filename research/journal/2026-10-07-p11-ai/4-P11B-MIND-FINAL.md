# P11b: AI object and update dispatch (FINAL)

Port: `hst_sim::ai::Mind` (state `Phase` Start/Serve/Receive/Rally, `active`, ALL-style `net` pick, `net_rate`,
`net_left`), `Mind::reset/count/own_hit/start/rally/play/point_over/point_result`, `Play` (Net/Base), `NET_RATE`
(15, 85), `serve_spot`, `serve_wait` (crates/hst-sim/src/ai.rs). Test `minds_match_the_game`
(crates/hst-sim/tests/ai.rs) against `context/fixtures/ai_mind_s05.bin` =
`tools/record_ai.py 5 5000 … 0x280 0x4230a8,0x423060` (record_ai.py gained an optional list of extra globals, 8 bytes
each, after the MT): 21 first updates, 6 net-rate steps, 74 own-team hit counts, 15 net picks, 21 reset/serve counts,
12 point-over stops, 6 serve spots+waits all as the game; the clamp bounds checked against context/ram/s05.bin.

## The original (doubles offsets; singles the same logic at other offsets)

- AI object at player+0x80; vtables singles 0x1d20e0, doubles 0x1d2100: [3] reset 3c8730/3ce4c0, [4] message handler
  3c8850/3ce7c0, [5] update 3c8c60/3cebf0. The player update calls update(ai, &stick, &buttons) for computer players;
  the AI drives a virtual pad.
- Top state +0x54: 0 start → on the first update 1 if server (0x42304c), 2 if receiver (0x423054), else 3 (no routine
  that frame). 1 serve (3cee70) and 2 receive (3cf7b0) return done when the stroke is over (player+0x3f95 == 0) → 3.
  3 rally: row style 1 NET → 3d02c0, 2 BASE → 3d1320, 3 ALL → NET if +0x274 else BASE. +0x60 == 0 zeroes the stick.
- Set state 3ced60: 3 → rally sub 0 (3d0110): roll(centre rate, the `position::Return` roll), then if counter
  +0x26c < 1: +0x274 = roll(+0x270), +0x26c = rand%3+2. 2 → receive sub 0. 1 → serve sub 0, which runs the timing
  draw 3d24e0(…,0).
- Reset (vtable[3], arg 0 only at match start): +0x60 = 1; at match start, if the partner has an AI object,
  +0x270 = 50, +0x274 = roll(50); then 3d24e0(…,0); state 0.
- 3d24e0's tail after the reaction: param 0 → +0x26c = rand%3+2; param 1 (hit message) → +0x26c −1 if the own team
  hit (362530); then the guess.
- Messages (broadcast 0x18b310): 0x11 toss → server re-enters state 1. 0x14 strike / 0x16 new ball path → state 3
  with sub ≠ 3 re-enters rally sub 0 (state 2 → receive sub 0). 0x15 hit → doubles re-enters rally sub 0 if state 3,
  shots > 1, sub ≠ 3; shot record; 3d24e0(…,1). 0x17 point over → +0x60 = roll(50). 0x19 reaction → ALL rows only:
  won (player&1 == 0x4230a8) with NET +5 / with BASE −5, lost the opposite; clamp [0x4179b8, 0x4179c0] doubles,
  [0x4178d0, 0x4178d8] singles (both 15..85); +0x274 = roll(rate).
- Serve sub-machine 3cee70 (+0x55):
  - 0 spot +0x70: serve level +0x5c == 0 → rand%3 → 0.7 / 2.0575 (singles) 2.7425 (doubles) / 3.415 (0x405a8f5b) or
    4.785 (0x40991eb9, one ulp off) else 0.7; × side (+0x12b0) × (−1 on the ad court).
  - 1: enter +0x58 = rand%60+60; stick ±1 while |x−pos| ≥ 0.033333, then count down to 0.
  - 2: toss kind +0xb1 (first serve 1; second roll(0x41e410[character] = 0,0,0,0,0,0,30,100,0,100,100,100,0,0);
    level 0 roll(10) → 2), press.
  - 3: contact search 360010 → frames +0xb4 (decremented each frame).
  - 4: when frames + serve error < 9: swing kind +0xb2 (toss 2 → 6, else roll(row 0x104) ? 3 : 4), press.
  - 5: aim during the swing (row 0x100, 50/50 rolls), done when the stroke ends.
- Slot-5 recording: each point state 0 one frame then [1,2,3,3]; AI 2 (ALL) stepped its rate 50→45→50 by its losing
  picks.

## In the app

`play.rs`: `Player.ai_mind` (kept across points), `ai_heard` (reset / point over / reaction heard), `ai_serve` (spot,
wait). `ai_update` runs ahead of `bot`: point reset with a timing draw + count, start state, hand-over to the rally
(serve: once served; receive: once the return is hit and the swing is over), point-over coin flip, ALL-style rate step
once the players react. `bot` dispatches the serve state to `bot_serve`, which now walks to the spot and waits 60–119
frames before tossing; an inactive mind stands still. `ai_heard_shot` re-picks in the rally and counts own-team hits.
`ai_draw` passes the smash-third case (doubles beside a human partner).

Ponytails: receive and both rally routines still run the stand-in loop (P11e/P11f); toss/swing kinds and serve level
(P11g) not drawn, so the usual 0.7 spot and strong toss; the mind isn't made afresh for a new match.

Smoke run: `HST_AUTOPLAY=1 target/release/hst "Hot Shots Tennis (USA).iso" --play` played a full game of bot points
including a fault and second serve.
