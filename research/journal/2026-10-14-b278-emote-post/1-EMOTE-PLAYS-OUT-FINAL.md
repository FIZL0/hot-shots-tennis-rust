# B278 — emotes play out past the point's end and after a fault

Cause: `emote::tick` only kept the lock in Rally/Serve. When the point ended mid-emote the phase went to Post,
the lock was zeroed and `locomote` put the player straight into the stand/ready stance; `reset_positions`
(next point's placement) also rebuilt the Player, dropping any lock.

Fix:
- `play/emote.rs`: emotes are open in Post too (point, fault or let call); a played-out reaction may be followed
  by an emote (`root.is_none() || played`).
- `play.rs react()`: the point's reaction clears the emote (the "next real action" in the post-point time).
- `play.rs reset_positions()`: a running emote's lock and motion command carry over the placement into the serve
  (the server holds the ball meanwhile, as B228).
- `play.rs control()`: `post_press` is taken before the emote check, so emoting in Post doesn't swallow the press
  that hurries the call.

Tests (`play::emote::match_tests`): `an_emote_locks_then_plays_on` now checks a rally emote counts down to its end
past the point's end (79-tick emote, point ended 38 ticks in, out at its full length, no reaction cut);
`an_emote_after_a_fault_plays_on_into_the_second_serve` whiffs 1P's serve (fault), emotes in the call, and checks
the emote plays its full length across the placement into the second serve, then the serve stance.

No match RNG draws or timing involved: the voice key is position-derived as before; with no flick the code paths
are as before (post_press is the same call, only earlier in the turn).

## Not verified / not 1:1
- Emotes are ours (B145), there is no original behaviour to compare against; checked by tests only, no PCSX2 run.
- Not visually checked with `--shot`: the motion played is asserted by `cmd.id` / drawn `Motion` in the tests.
- An emote at the change of ends is still cut (phase closed there); not part of the task.
- After an emote that follows a played-out reaction, the body holds the emote's last frame until the next point
  (nothing re-sets a motion while the reaction phase holds the players).
