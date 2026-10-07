# P11e3: guessing (ヤマ張り) (FINAL)

Port: `AiParams::guess`, `Guess::target_x` / `Guess::verdict` (crates/hst-sim/src/ai.rs); `Timing::reacted`. Tests
(crates/hst-sim/tests/ai.rs): `guesses_match_the_game` (ai_s05.bin: all 8 serve draws seen by the receiving bots,
3 guesses, side and move frames exact, right after the timing draw) and `guess_verdicts_match_the_game`
(`context/fixtures/ai_guess_s05.bin` = `tools/record_ai.py 5 4000 … 0x260`, which adds the saved spot at +0x250: the
one guess that reaches its verdict in it comes out "neither", as the game's receive state shows).

## The original (doubles offsets; singles 3cbbe0 is the same at −0x10 without the partner check)

- Tail of the per-shot draw 3d24e0, after the serve error. Draws in between, in order: quick serve (+0x244), dive
  (+0x245: only at a reset or after its own smash), body shot (+0x24c), low shot (+0x24d), four level picks
  (+0x5c..+0x5f), then if an opponent hit last: the reaction (+0x248, one draw) plus a lob (shot kind 1–3 with
  record +8 = 3) or special-serve (kind 0 with record +0xd) draw; at a reset the +0x26c draw. On an opponent's serve
  that is 8 draws, 9 for a special serve. `guess` takes them and drops them until P11c/P11g/P11h port them.
- Guess (+0x24f): 0 if the pace or fast-ball reaction took hold (the function's flag, set even for a 0-frame pace),
  or the opponent didn't hit last, or shots this rally (0x423060) > 1 (so only the serve), or the partner
  (AI +0xec) is human (player +0x13f0 < 0x20); else roll(row 0x10c), then roll(50) → 1 else 2. A guess sets the
  reaction +0x248 = row 0x110 and saves the player's position (+0x3d70) at +0x250.
- Receive state 3cf7b0, substate 0: while +0x248 > 0 it counts down; guessing, it runs to (x, own z): x =
  side·5.485 for guess 1 on the deuce court (0x423050 = 0), −side·5.485 for guess 2 on the ad court, else 0; the
  step function 35c970 zeroes the pad within 2/3 of a step, which ends the wait (+0x248 = 0).
- When the contact search then finds the ball: dir = side (guess 1) or −side; d = normalised (contact − saved)
  in xz · (dir, 0). d ≥ 0.5 → right: +0x24e = 1, state 1 (the swing state 2 then takes err = rand%3 − 1, and the
  approach run is pushed ×2, ×1.5 when +0x24c). d < 0 → wrong: pad zero, +0x248 = row 0x114, back to state 0.
  Else state 1. The guess is cleared either way; an own-team hit's redraw also clears it (2 of the 3 guesses in the
  slot-5 recording).
- Slot-5 rows: 84–86 guess (15, 8, 32); 89 (100, 16, 0).

## In the app

`play.rs`: `ai_draw_guess` after each opponent-hit redraw in `ai_heard_hit`; `ai_guessing` in `bot()` holds the bot
(running to the guess target, or stuck) and judges the guess on the first frame it has its contact point. Ponytails:
no special serves (draw count 8), the right-guess run boost waits for P7f, the stop distance is the bot's 0.1 m.
Rolls use the app's xorshift.

The app's `--play` doesn't start in this tree (B0: voly table panic, fixed on its own branch), so no in-app smoke run.
