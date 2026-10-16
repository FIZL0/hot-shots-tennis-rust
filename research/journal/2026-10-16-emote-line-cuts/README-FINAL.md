# B473 — emote lines cut each other and give way to reactions

Emote voice lines went through `Game::whooshes` like every other one-shot, so nothing ever stopped them: an emote
started as soon as the last one's motion ended, while its voice line could still be sounding, and the post-point
reaction voice played over a line from the rally.

Now emote lines and the post-point reaction voices (`reaction_voices.rs`) go on their own queue, `Game::lines`
(`emote::Line::{Emote, Reaction}`), and `emote::play_lines` (FixedUpdate, after `play_sounds`) plays them at their
player. It keeps each player's last emote line's play id (a `Local`, presentation only): an emote line stops its
player's previous one, a reaction line stops every player's. Each line plays once per tick however often a
rollback re-runs it (`Played::first`, as `play_sounds`), and the cut happens only when the line really starts.
Nothing draws from the match's generators or changes the sim; the queue is drained each tick.

Test: `emote::tests::a_new_line_cuts_the_last_emote_line` (two real emotes by 2P, then a reaction line).
Ran emote, reaction, sound, snapshot and netplay (`HST_NETPLAY=1`) filters: pass.

## Not verified / not 1:1
- The original has no emotes (they're ours), so there's nothing to compare the cut against; presentation only, per
  PLAN. Not listened to in the running remaster (no audio check unattended); the test checks the stops on the mixer.
- The other voices (serve calls, shouts, whiffs) don't cut emote lines: the task names only emotes and post-point
  reactions.
