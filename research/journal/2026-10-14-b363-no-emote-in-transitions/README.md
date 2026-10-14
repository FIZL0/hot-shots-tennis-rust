# B363 — no emotes in a won game's or set's transition

User 2026-10-14: players shouldn't emote during point transitions such as the game-won / set-won moments.

- `play/emote.rs` `tick`: in `Phase::Post` the emote is now closed when the post-point's event (`PostPoint::event`)
  is `Game` or `Set` (a set covers the match's last point too). An emote under way is cleared, as in `ChangeEnds`.
  An ordinary point's post-point time and a fault's or let's call stay open (B278 asked for those). The change of
  ends was already closed. Ours, layered on top: emotes don't exist in the original, so with no flick nothing changes.
- Test `no_emote_once_a_game_is_won`: the B278 fault setup, with the post-point's event switched to Game, then to Set,
  just before the flick: no emote starts. Left as the fault (None), the same flick starts one.
- `controls.rs` and `hst-sim/src/flow.rs` didn't need changes.

## Not verified / not 1:1
- The original has no emotes, so there's nothing to compare against. The phase that counts as "a transition" is my
  reading of the request: only won games and sets. Ordinary points stay open because B278 asked for them.
- The test switches the post-point's event by hand instead of playing a whole game to its winning point (that's
  quicker, and the check only reads that field).
