# N5a — hitting a lob off a lob

Done. The contact search, the △ smash launch and its flight match the game on a recorded rally. The one port
bug found is a doubles double-strike, now prevented.

## Recording

`tools/record_lob_smash.py 5 context/fixtures/lob_smash_s05.bin <frames>`: the slot-5 bot doubles game. Whenever
a player's search locks a smash (+0x3ec1 == 4, countdown +0x3ec4 ≥ 0), it pokes the button +0x3ee4 = 4 (△)
before the hit, so the game's own hit code launches smash kind 1. 3628 frames (the run was cut by a timeout while
waiting for PCSX2). Three △ smashes:

- vsync 8422, p1; p2 volleys it back at 8467;
- vsync 8902, p2; p3 returns it off the bounce at 9003;
- vsync 10451, p0, off a lob (class 1 kind 3); p3 returns it at 10550.

## Verified

- Contact search (`swing.rs lob_smash_contact_search`): all 38 decisions match, including the three returns of a
  lob smash and the smash decisions themselves. Same frame, branch, side and ball.
  - One stroke at 9299 has the game's stored contact (+0x3f40) 1.2e-5 off its own flight. The port's prediction
    equals the ball's real position at the contact frame, so the test accepts that.
- Launch (`shot_tables.rs lob_smashes_launch_like_the_game`): all 3 exact.
  - Inputs: table `smsh1`, `Bounds::smash(1)`, the hit's timing scatter (zero here).
  - Velocity matches to ≤ 2e-7; flight frames are exact.
  - Spin is 5°, the same as kind 0. play.rs's guess is now confirmed.
  - These smashes are slow (0.25 m/frame), so the test's serve-ball speed cut dropped from 0.3 to 0.1.
- Flight (`swing.rs lob_smash_flights`): bit-exact from launch to the first bounce (44, 67, 73 frames).
  - Ball fields at launch: curve, bend, first-bounce spin/restitution and turn are all 0, as the app's `strike`
    sets them.

## Found, not fixed

- Bounces in these live recordings all land ~1e-6 off, ordinary strokes included (0 of 115 first bounces exact
  with `COURTS[10]`). Velocity and spin after the bounce are often exact while the position isn't. This is
  general, not lob-specific; probably the unverified court index for slot 5 (see REFERENCE).
- Stroke and smash timing scatter (+0x3ecc × +0x3ee0 − +0x3edc, ×1.5) is still not applied in the app's `strike`.
  It is zero in nearly every recorded bot hit.

## Port fix

The "illegal hit" in the report is the judge's call 6: one team hitting twice. In doubles the app let a human
and the stand-in bot partner (or two pending presses) both lock onto the same ball, then both struck.

- The original never has two teammates locked at once: 0 frames in match_s05 or lob_smash_s05.
- `play.rs theirs()` now gives `find_contact`/`find_dive` nothing while the partner holds a contact or dive.

The game's own cancel flag +0x3ec8 (countdown ends in a whiff) is set by event 0x19, raised by 0x326270. That
looks like the rally-end path, not a partner hit.
