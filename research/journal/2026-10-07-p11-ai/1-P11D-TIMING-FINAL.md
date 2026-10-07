# P11d: timing errors (FINAL)

`AiParams::timing` (crates/hst-sim/src/ai.rs) draws the AI's stroke/volley/smash/serve timing errors with the
change-of-pace and fast-ball terms. It is wired into `play.rs` bot/bot_serve, which drops the stand-in timing tables.

## The original

- Per-shot draw: 0x3cbbe0 (singles), 0x3d24e0 (doubles). The doubles layout is the singles one shifted +0x10.
  Errors are at singles +0x224 stroke, +0x228 volley, +0x22c smash, +0x230 serve; the flag +0x220 halves the stroke
  error. In doubles these are +0x234..+0x240 and the flag is +0x230.
- Shot records are 0x40 bytes: +0 hitter (−1 none), +4 kind (0 serve, 1 ground, 2 volley, 3 smash, 4 dive), +0x30
  velocity. Doubles: 0x130 last opponent shot, 0x170 the one before, 0x1b0 last serve, 0x1f0 serve before. Singles
  has them at −0x10.
- Hit message 0x15 (doubles handler 0x3ce7c0):
  - On an opponent's hit it shifts 0x130 → 0x170 and stores the shot. A serve also shifts 0x1b0 → 0x1f0.
  - On its own team's hit it clears the flag.
  - Either way it then draws with arg 1.
- Draw order, on MT19937 0x427130. A chance roll is `(r>>16 & 0x7fff) % 100 < p`.
  1. Rally pace: needs the shot before with kind ≠ 0, and a roll on pace[3] passes.
  2. Otherwise serve pace: needs the last shot is a serve, this AI is the receiver, there is a serve before it, and
     a roll on serve_pace[3] passes.
  3. pct = (int)((big/small − 1)·100). If pct ≥ p[0], pace = min(p[1]·(pct−p[0])/5, p[2]) × sign (+ when the last
     ball is faster). The speed is summed y² first (`mula y,y; madda x,x; madd z,z; sqrt`).
  4. If the flag is clear, roll react_speed[2]. If (kind ≠ 0 or receiver) and 3600·60·speed/1000 > react_speed[0],
     the fast-ball reaction is react_speed[1] (its use is P11c's).
  5. Errors: a sign bit (r>>16 & 1) then size `% (n+1)`; the pace adds to the size and sets the sign.
     - Stroke (halved under the flag), volley.
     - Smash = base + rand % (rand_n+1). Doubles uses base/3 when ai+0x28 ≠ 0.
     - Serve (no pace).
     - The quick-serve, dive etc. draws that follow belong to other subtasks.
- Use: at the swing decision the error for the contact kind (+0x9a: 0/1 stroke, 2 volley, 3 smash, 4 dive → 0) goes to
  +0xb8. The AI presses when frames_left (+0xb4) + err < 9, i.e. frames + err ≤ the sweet frame. With +0x23e set,
  err = rand%3 − 1 instead (left out). The serve uses its +0x230 the same way.
- Reset (0x3ce4c0) with arg 0 clears the serve records, sets the flag to 1 and draws. The arg is 0 only when gm+0x344
  (close-up) == 0 and 0x423040 ≠ 0 (set at match start, 0x325bc0 clears it). So the flag is "own team hasn't hit
  since the match began". In slot 5 it was 1 at the start and stayed 0 after later point resets.

## Verification

- `tools/record_ai.py 5 4000 context/fixtures/ai_s05.bin` records 4000 frames of the slot-5 doubles bots: globals,
  the MT, and the 4 AI objects each frame.
- `timing_errors_match_the_game` (crates/hst-sim/tests/ai.rs): for every change in a doubles AI's errors, it rebuilds
  Shots from the records and searches the MT draw offset.
  - Result: 155 draws exact, including 4 with a change of pace and 3 fast-ball reactions.
  - Paced cases are also checked not to match without the shots.
- Singles uses the same code at −0x10 (no third-of-smash case). It is unverified because no singles bot state was
  recorded.

## In the app

- `Player` has new fields: `ai_seen`, `ai_serves`, `ai_hit`, `ai_timing`.
- `strike` calls `ai_heard_hit` after the flight is set. It maps branch → kind 0,1,2,dive 4,smash 3, then every bot
  remembers the shot or notes its own team's hit, and redraws.
- `bot()` presses when `frames + err ≤ SWEET_FRAME`, with err picked by branch. `bot_serve` draws first and uses
  `SWEET_FRAME − serve`.
- Rolls use the app's xorshift, not the game's MT. smash_third is always false (ai+0x28 is P11b's).
- The fast-ball reaction is drawn but not used (P11c).
