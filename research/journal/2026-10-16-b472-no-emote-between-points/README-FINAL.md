# B472 — no emotes between points

Emotes (B145, ours) could still be started in the post-point time: B278 had opened `Phase::Post` on purpose
(a fault's or let's call included, an emote carrying through the placement into the next serve).
B472 reverses that: emotes start in live play only.

## Change
`play/emote.rs` `tick`: a flick starts an emote only outside `Phase::Post` (changes of ends were already closed).
An emote that began in the rally still plays on into the post-point time until it's out or the point's reaction
ends it (B278's other half, so a flick as the point ends doesn't snap the motion); after a game/set point it is
cut as before (B363). Live play = rally, waiting for a serve, the server before its toss — unchanged.
`tick_input.rs` (named on the task line) needs no change: the flick stays on the wire, the gate is the match's.

## Tests
- `no_emote_between_points` (replaces `an_emote_after_a_fault_plays_on_into_the_second_serve` and
  `no_emote_once_a_game_is_won`): a flick in a fault's post-point time, and in one turned into a game/set win,
  starts nothing; held through the whole post-point time still nothing; a fresh flick in the second serve's wait
  emotes. Fails with the gate removed.
- `an_emote_locks_then_plays_on` still passes (rally emote playing into the post-point time).

## Not verified / not 1:1
- Emotes are ours (not in the original): nothing to compare against PCSX2. Not checked by eye in the app.
- The placement's emote carry in `play.rs` (B278) is now normally unreachable (the reaction always ends an emote
  first); left in place so play.rs stays untouched for the parallel merges.
