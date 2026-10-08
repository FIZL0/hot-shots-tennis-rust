# P11l3: singles dispatcher wired into the app (final)

## The original

- **Dispatcher 0x3c8c60.** Same shape as doubles (0x3cebf0):
  - state 3: NET 0x3ca4e0 / BASE 0x3cb130 by style, the ALL coin at ai+0x260;
  - state 2: receive 0x3c9a40 (true → state 3);
  - state 1: serve 0x3c8f10.
  The state setter 0x3c8e00 calls 3 → 0x3ca3d0(0), 2 → 0x3c9980(0), 1 → 0x3c8e80.
- **Message handler 0x3c8850:**
  - **0x14/0x16** (strike/new path): as in doubles. State 3 with sub ≠ 3 → set_sub(0); state 2 with receive state ≠ 3 → set_state(0). The 0x16 shot-1 hold and the phase-3 clear of +0xbc are the same.
  - **0x15** (hit): no set_sub(0), unlike doubles. It does three things:
    - copies the shot memory: the opponent's shot to 0x120.. (kind at 0x124, spot at 0x130), the previous one to 0x160.., and its own to 0xe0..;
    - on an opponent's kind-3 shot (a dive), a NET or ALL style that isn't dashing dashes: +0x256 = 1, stand x = 0, stand z = −reach·side;
    - then the timing draw 0x3cbbe0(1).
  - **0x19:** net_rate ±5 at 0x25c (clamped), net = chance at 0x260.

## Wiring (`play/doubles_ai.rs`, `Rally::enter_rally` / `heard_hit`)

- `step` now also runs with 2 players. The partner is itself, the opponent is opp[0], and the centre rate/radius are the singles ones. It also sets the high level, the opponent's hand, and the shot target.
- `enter_rally` uses the singles set_sub(0) in singles. The rally re-entry fires on each new shot (the new-path message), not only after shot 1.
- On an opponent's shot, `heard_hit` gets the kind (the app's `ai_seen`), and the mark is the hitter's position.
- The rally runs `singles_rally`; the receive is the shared `receive` with `singles` set.
- **Run:** `HST_AUTOPLAY=1 hst <iso> --stage 1 --play --singles`, 3 min.
  - About 15 points, rallies up to 16 shots.
  - Buttons: 44 ✕, 4 ○, 13 △.
  - Kinds 0–4 all seen, the dash engaged, no panics.
- **Check:** `replay()` asserts the dispatch for every singles call (net byte 0x260). It passes on all three singles fixtures.

## Gaps → P11l6

- The shot target (+0x3e90) is where the app's flight lands, not the aimed target.
- The shot memory's spot (0x130) is the hitter's position, not the ball's.
- The reach's under_min is 0, because the app's `Reach` lacks it.
- Each new shot counts as both the strike/new path and the hit, in that order.
- Plus the doubles wiring gaps that apply in singles (P11k6: the contact depth, the smash offset, the path mode and gap, the hold flag).
