# P11h: AI dive and call-out (final)

Fixture `context/fixtures/ai_dive_s05.bin`, from `tools/record_ai_dive.py 5 15000` (slot 5, the bot-only doubles
match). Every frame holds the score globals, the AI's ball path, each player's side, reach block, facing and position,
run velocity, dive flag and locomotion state, and the first 0x280 bytes of its AI object. Test: `ai_dives_match_the_game`.

## Dive

- The rally routine tries a dive only after its stroke searches find nothing and the partner doesn't take the ball.
  Three conditions must hold:
  - the AI drew the dive chance (`AiParams::picks`' dive);
  - the game isn't in the mode that never dives;
  - the dive check finds the ball.
  The AI then plans a dive: it aims, presses its shot button, and the player's press finds no stroke and dives.
- The dive check runs the same scan as the player's dive search, `swing::dive` (hit or miss candidate). It needs the
  player to be running and scans path steps 5 up to min(path frames left, 16).
  - The path is this frame's, but the player state is the previous frame's: at the check, the player hasn't moved yet
    this frame.
  - In the AI's path buffer y is up, and the buffer starts at step 5.
- The dive chance is drawn only at a reset or serve, or after the AI's own dive; otherwise the last draw is kept.
  `play.rs` `ai_draw` now carries an undrawn chance over.
- Verified: every AI dive in the recording had the chance drawn, started from a run, and `swing::dive` finds the ball
  on the AI's own path.

## Call-out (partner hand-off)

- The call-out flag is set at a reset and whenever the opponent hits. When the AI hands the ball to its partner, the
  flag is used up:
  - if the partner is mid-stroke or diving (locomotion past running), there's no call;
  - otherwise there's a 25% roll for the call (`sound::call_out`, already in `play.rs`).
- The hand-off comes only after the AI's reaction hold has run down, and only from the rally's second shot on (the
  rally substate returns early before then).
- Verified: every flag clear is a hand-off (or happens while the partner is busy), and none falls during a hold.
  `play.rs` `bot` now gates the call on `ai_hold == 0`, `shots >= 2` and `!ai_mate_busy`.

## Left open

- `ai_dives`' "nothing in reach" uses the app's press search and approach, not the original's per-level stroke
  search. That search belongs to P11i (the bot-only match the AI logic is checked against).
