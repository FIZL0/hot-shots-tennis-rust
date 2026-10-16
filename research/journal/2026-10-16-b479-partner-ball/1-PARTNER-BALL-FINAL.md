# B479 — can a teammate take a ball its partner dives for?

Yes. The port said no and now matches the original.

## The original's rule

- The contact search on a press (the human/AI press entry; branches smash → volley → ground → dive) only checks
  that the ball is in play and that the **other team** hit it last (`last hitter & 1 != own team`). It never looks
  at the partner: no lock, dive or claim of the partner gates it.
- The dive branch has two outcomes. A path point within the dive's reach (run speed × frames + 1.3 × reach) locks
  a countdown. A point within **twice** that range (facing ±22.5°, height 0.2–1.8 × the character's) starts the
  dive anyway: branch 3, countdown −3, +0x3fa0 = 999, no lock (a dive that falls short).
- What stops a double hit: every strike/hit/net-cord message (0x14/0x15/0x16) sets **each** player's
  contact-cancel flag +0x3ec8. A countdown that reaches 0 with the flag set plays the miss (0x350840) instead of
  striking. For the dive motion 0x1e that has no miss motion, so the dive just slides on.

## Recording

`research/b479_partner_ball.py 4 dive 120 4.0 10` (slot 4, P1 human beside a CPU partner): on the opponent's hit,
P1 is put level with where the ball crosses the partner's z, 4 m out plus its run, runs at it and presses ○
10 frames before the ball crosses. Log: `context/b479/dive-4.0/log.txt`.

- v7901: P0 dives short (branch 3, countdown −3, motion 0x1e) and is still sliding until v7916+.
- v7904: P2 (the CPU partner, at the net) locks a volley (branch 2, countdown 2), and at v7906 it strikes
  (last hitter 2, shot 3). At that strike P0's cancel flag goes 0 → 1.
- With gap 3.0 (`context/b479/dive-3.0/`) P1 reached a stroke instead. The `lock` mode (P1 presses while the partner
  is locked) never got a window: the CPU's net volleys lock only 2 frames before contact.

## Port

- `play.rs theirs()` lost its partner gate. N5a had added it because a human and the stand-in bot both struck the
  same ball, the illegal hit. That was before B128's `cancel_locks`, which now resolves a double lock as the
  original does: the first strike cancels the rest.
- `cancel_locks` also turns off a dive's contact: a reaching dive cancelled by a hit (or a net touch) slides on
  without striking, as the original's flag on the dive's countdown.
- Test: `play/partner_ball.rs` `a_partner_takes_the_ball_a_dive_falls_short_of` (four bots). When the other team's
  return comes in, player 0 is put into a dive falling short (found with the port's own dive search) and player 2
  beside the ball's path presses. Player 2 locks and strikes while player 0 is still sliding (it fails with the old
  gate: "player 2 can't lock while player 0 dives"), and a hit clears a reaching dive's contact.
- Full `tools/check.sh` passes (761 tests before the new one; replay/bot tests unchanged: the stand-in bots pick
  one teammate per ball (`mine`), so they never double-lock).

## Not verified / not 1:1

- The cancelled reaching dive is not recorded. It follows from the decompile (motion 0x1e has no entry in the miss
  routine), but no capture shows a partner's hit landing during a dive that would have reached the ball. Forcing
  one needs the CPU to lock in the dive's last frames, which the 2-frame net volley locks didn't allow.
- Two teammates locked at once (two humans) isn't recorded either: the slot-3/4 saves have one human, and the
  CPU's locks were too short to press into. The port's resolution (first strike, others cancelled) is B128's,
  which was checked against net-cord captures.
- The test drives the scenario by placing the players rather than replaying the recorded inputs. The recording
  is the original's answer to the question, and the test checks the port gives the same answer, not frame-exact
  positions.
