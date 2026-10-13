# B231 — emotes while serving play their motion

## Cause

Since B228 the server may emote before its toss: `emote::tick` sets the emote's motion command, voice and ball hold.
But the draw-side `play::motions` system has a serving branch that, with no toss yet, plays the baseline walk's
motion (0x20–0x22) and otherwise forces the stance 0x20 every frame — so the voice played while the body stayed
in the serve stance.

## Fix

`play.rs` `motions`: the serving branch is skipped while `players[i].emote > 0`, so the emote's command plays
through the generic path; when the emote runs out the branch takes over again and the stance (0x20) comes back.
Draw-side only: nothing in the sim, no generator draws, serve timing untouched (the emote lock itself is B145/B228's).
A toss already started isn't interruptible: `emote::tick` closes the window once `serving.toss` is set (unchanged).
The receiver's emote never went through this branch (B145's test covers it).

## Tests

- `play::emote::match_tests::the_server_emotes_before_the_toss` now spawns the two figures and checks the drawn
  `Motion` plays the emote for its length and is back in the stance two ticks after. Fails without the fix
  ("the body kept the stance, tick 10": 0x20 vs 0x2c).
- Online: `motions` is draw-only, so the host-emote rollback test (`the_hosts_emote_plays_on_both`, B228) is unaffected.

## Not verified / not 1:1

- Emotes are ours: nothing to compare against PCSX2.
- Not looked at in a `--shot` or with two real online instances; checked through the drawn `Motion` id in the
  in-process match test only.
- The switch back to the stance is a plain `Motion::play` (whatever crossfade `play` does), not tuned by eye.
