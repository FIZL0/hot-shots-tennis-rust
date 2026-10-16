# 23 — P11k4: the receive's last branches, from P1's serves (vpad)

The CPU's receive (state 2) only ever answers a serve, so these branches need P1 to serve. P1 is human in slot 3,
so `tools/record_ai_rally.py` drives P1 from the pad (`HST_DRIVE=1`, tools/vpad.py serve with `HST_PCSX2=3`).

## Driving the serve

`HST_SERVE=<buttons>` adds a serve driver. Findings from the pad:
- The first press after a load does nothing. The next press tosses. The hit has to be pressed while the ball is
  still rising: pressed on the way down, the swing misses (the point stays at 0 shots and the ball drops back into
  the hand).
- If the stick is held before the toss, the server walks along the baseline.
- The swing clears the serving state (+0x3ec1 = 5) at once, but the stick still has to be held after the hit
  press. Releasing it there gave plain serves.

What the driver does now:
- Optionally walks out (`HST_SERVE_WALK`; walking gave worse angles, so it's off for these fixtures).
- Presses the toss every 12 polls (`HST_SERVE_DELAY` holds off the first serve).
- Holds the stick wide (`HST_SERVE_X`/`_Y`) from 5 polls after the press, through the hit and for 20 more polls.
- Presses the hit when the rising ball passes a height that cycles between 1.6 and 2.6 m.

✕ with the stick at (1 toward the receiver's sideline, 1 down) is a short slice. It lands in, and the receiver is
at |x| ≈ 6.2 when it bounces and ≈ 6.9 when it aims. That puts the receiver past the 5.485 line by 50 cm or more,
so the out-of-court roll is 20%.

## Fixtures (`ai_rally.rs vpad_serves_receive_matches_the_game`)

All are slot 3, tag 1 (receive), `HST_DRIVE=1 HST_SERVE=cross HST_SERVE_Y=1 HST_SERVE_WALK=0`. Every call replays
bit for bit. The test reads from the game's own exits that each branch ran.

| Fixture | Extra | Frames | Calls | Reaches |
|---|---|---|---|---|
| `ai_rally_vpad_wide.bin` | `HST_SERVE_DELAY=61`, 1500 | 1532 | 885 | plan 11 ×2 |
| `ai_rally_vpad_wide_b.bin` | `HST_SERVE_DELAY=47`, 1500 | 1522 | 1100 | plan 10 ×1 |
| `ai_rally_vpad_last.bin` | `HST_PATH_STOP=9`, speed 0.3, 2000 | 2029 | 1276 | last-entry pick ×11 |
| `ai_rally_vpad_smash.bin` | `HST_CHAR=k:0xe0:0.01,k:0xe8:1.8,k:0xec:1.2,k:0xf0:0.8` for k = 1, 3; `HST_SERVE=cross,circle`, 3000 | 3033 | 1587 | smash find ×4 |

The counts were checked two ways: with temporary eprintln markers in `rally.rs` (since removed), and by the test's
own checks. The test's checks are:
- plan 10/11 set on the aim (state 2 → 3);
- a copy of 1..=11 entries in state 0 (for the last-entry pick);
- state 0 → 1 with kind 3 (for the smash find).

Both ways agree.

Every slot-3 load replays the same first serve: plan 11, receiver at x −6.93. A long natural run (15000 frames,
12 P1 serves) only gave plan 11 twice. `HST_SERVE_DELAY` shifts the AI's draws, so short runs with different
delays each get a new roll; delay 47 gave plan 10.

## Why two branches need a poke

- **Last-entry pick.** The path object's step (0x35dd40) does two things: before the second bounce it adds 14–15
  entries a frame, and after it, it tops up whenever fewer than 15 are left ahead. It stops only at its
  600-entry cap (+0x5c = 0), and the messages that end a point turn the path off (0 entries). So the AI's copy is
  either 0 or at least 15 entries long (every non-empty copy in these fixtures was ≥ 15). The pick (no entry 11)
  would need a receive still running 600 frames after the serve, and no point lasts that long.
  `HST_PATH_STOP=9` clears +0x5c once per serve when the ball passes z 9 on the receiver's half. That is the state
  the cap would leave. With players at 0.3 speed, the search misses and the fallbacks walk the copy down to its
  last entry.
- **Smash find in the receive.** The search needs every tier to miss while a falling, once-bounced ball is between
  the smash low and high heights. The smash low is 1.9 m (character 0x7ca1c0) or 2.2 m. Tier 3 covers up to
  min(3 × stroke height, base + far) = 2.25–2.55 m. And no serve the pad could make kicked above 1.6 m after the
  bounce (✕, ○ and △, short and long). `HST_CHAR` pokes the two receivers' character records: stroke height 0.01
  so the tiers miss, smash window 0.8–1.8 m. The records carry the 0x300 region, so the replay uses the poked
  values.

The out-of-court aims (plans 10/11) need no poke.

## Not verified / not 1:1

- **Last-entry pick and smash find:** proven only on poked states, a stopped path and poked character heights.
  The branch code and its inputs are the game's and replay bit-exact, but no unpoked recording reaches them. The
  reasons are above: the path never leaves 1..11 entries, and serves don't kick into the smash window.
- **Singles receive:** not covered here (it is P11l4/P11l5). Its out-of-court aim, plan-10 upgrade and special
  return are still unreached. The same serve driver should work with a human singles save; there is none (slots 3
  and 4 are doubles).
- **Pad timing is wall-clock,** so a re-recording won't serve the same points. The fixtures are what's checked.
