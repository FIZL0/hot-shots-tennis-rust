# B228 — online: the host can't emote

## Cause

The input was never dropped: the host's flick goes out in its pad's right stick bytes like any other press
(`tick_input::encode` → `netplay::wire` → ggrs → `unwire` → `decode`). A new two-peer rollback test that has the
host flick (`play::netplay::tests::matches::the_hosts_emote_plays_on_both`) played it on both machines before the fix.

What kept the host from emoting was `emote::tick`'s window: a rally, or the serve phase *for the receiver only*.
Online the host is always 1P, and 1P serves the first game, so the host's whole wait before each serve was closed
while the guest, receiving, could emote all along. Rallies are short and mostly mid-stroke (`idle` fails), so it
looked like the host couldn't emote at all.

## Fix

`play/emote.rs`: the server may emote too, until it tosses (`Game::serving.toss` is `None`). While it emotes the ball
stays held (`hold_ball`, and `held_ball` keeps it at the emote motion's left hand); after the emote the serve walk
and toss carry on as before. No play.rs or netcode change. Still input-driven only and no generator draws: a match
with no flick runs exactly as before (replay tests untouched).

## Tests

- `play::emote::match_tests::the_server_emotes_before_the_toss`: 1P human serving flicks at tick 10: the emote
  plays its full length in the serve phase, the ball stays within reach and still, the next ✕ tosses.
- `play::netplay::tests::matches::the_hosts_emote_plays_on_both`: rollback doubles at 5 ticks' lag, the host's 1P
  flicks from its first serve: the emote starts on host and guest, both ran the same pads, no desync.

## Not verified / not 1:1

- Emotes are ours (no original to compare to); nothing checked against PCSX2.
- Not tried with two real `hst` instances online; only the in-process two-peer harness.
- The held ball rides the emote motion's `Bip01LFinger21`; not looked at in a `--shot` (an emote that swings the
  left arm wide carries the ball with it).
