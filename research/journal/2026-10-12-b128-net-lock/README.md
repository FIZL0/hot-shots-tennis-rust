# B128 — a ball that touched the net could still be hit

## What the original does
- Judging a net cord (dead on the hitter's side, live if it rolls over) and keeping it out of the contact
  search were already ported in P12c: a net touch re-seeds the predicted path, but the contact count it keeps
  is past every search's limit, so no *new* lock happens after a touch.
- The gap was a swing that had **already locked** before the touch. The player's message handler sets the
  contact-cancel flag (player +0x3ec8) on 0x14 (body hit), 0x15 (ball hit), 0x16 (net touch, sent by the ball
  step) and 0x19 (point over). A new lock clears it.
- When the lock countdown reaches 0 with the flag set, the strike never runs. Instead the miss routine plays:
  the swing switches to its miss motion (0x10–0x19 → 0x27/0x28, 0x1f/0x25 → 0x29, 0x26 → 0x2a), the "missed"
  flag is set, the miss voice plays, and there's a re-press lock (30 frames for a smash, else 2).

## Proof (PINE, `research/b128_net_lock.py`, copy 4, slot 5)
The script waits for a rally lock, then puts the ball at the cord, flying toward the locked player.
- `context/b128/lock1.log` (`5 1 4 0 0.93 0.15 150`): P1 locked a ground stroke with countdown 10. The ball
  touched the cord 2 frames later (special count 1). The countdown ran 9…1, then −1, skipping 0, so there was
  no strike: the shot count stayed put. The ball dropped back on the hitter's side, bounced, and the point
  ended.
- `context/b128/roll1.log` (`5 2 6 0 0.96 0.3 120`): P0, at the other end, locked with countdown 10. The touch
  came 1 frame later, and again 9…1, then −1, with no strike.

## Port (`crates/hst/src/play.rs`, additions only)
- `Contact.cancelled` is set by `cancel_locks`. It runs on every frame the ball's special-material touch count
  goes up (`simulate`), and for everyone but the hitter in `strike` (0x15).
- `advance_stroke`: at the contact frame, a cancelled lock calls `miss_contact` instead of `strike`. That plays
  the miss motion, the miss shout, and a `motion::Whiff` placed at its pose frame, so the re-press lock and the
  recovery are the whiff's.

## Not verified / not 1:1
- 0x14 (body hit) and 0x19 (point over) don't cancel locks in the port yet. A body hit ends the point, so the
  lock's strike can't score, but the swing would still hit rather than whiff. Not recorded.
- The recordings don't show which motion was played or whether the miss voice played (only the countdown and
  shot count were logged), so the miss motion/shout comes from the decompile only.
- A touch on the exact frame the countdown reaches 0 depends on the order of the ball step and the player
  update. Not recorded.
- The re-press lock uses `Whiff::lock`, which gives 30 only for anim 0x1f. The original gives 30 for any smash
  branch (anim 0x25 too).
- No replay fixture has a net cord during a lock, so the replay tests don't cover this. The two PINE logs above
  are the proof. They're one-off captures, not an automated test.
