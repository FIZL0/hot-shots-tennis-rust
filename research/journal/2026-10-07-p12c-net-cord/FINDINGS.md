# P12c — net cord in a rally: point to the hitter (done)

The judging was already faithful. The port's contact search was not.

## Judging (0x1a9020 check, 0x378080 call) — no change
Calls are 0 none, 1 In, 2 Out, 3 Net, 4 NetIn, 5 NetOut.
- NetIn on the serve is a let. NetOut on the serve is a fault.
- In a rally, NetOut gives call 5 and the point goes to the other side.
- In a rally, NetIn lets play go on until 2 court contacts, and then the point goes to the **hitter**. This is
  the original's behaviour, not a bug.

I checked this with a scratch replay of the vsync 15010 sample of `net_s05.bin` stepped through `step_world` and
`Rally::check`, nudging vel.y between −0.02 and −0.0105:
- −0.018 gives NetIn, and then the point goes to the hitter.
- Most other values give NetOut, and the point goes to the other side.
- −0.0185 and below give Out.

## Why nobody plays a net cord that drops over
The original's predicted path is the ball at `*(gm+0x98)`. It is stepped against the court plane only, never
the net, and filled by 0x35dd40 → 0x3786a0.
- Each entry is 0x30 bytes: position, velocity, and contacts (+0x228; the net touch does not count).
- The hit (gm message 0x15) fills entry 0 plus 14 steps. After that, 15 steps are added each frame until
  the last entry has 2 contacts. The recording confirms this: predictor frame = 15(t+1)−1.

At the end of the ball step (0x375e30), if the live ball touched a special material that frame (and +0x8f4 == 0):
1. The predictor's +0x224 and +0x228 are saved.
2. Live ball bytes +0x50..+0x26c are copied into the predictor, and 0x375da0 re-launches it.
3. **The old +0x224 and +0x228 are restored.**
4. The game sends gm message 0x16. Its handler 0x35e150 restarts the path and resets the 4 locked contact slots.

A rally net cord comes about 20–40 frames after the hit. By then the old prediction already has 2 contacts, so
every re-seeded entry has ≥2 contacts.

`swing::search` breaks on `bounces > max_bounces` (1 for a ground stroke or smash, 0 for a volley), and a dive
needs ≤1. So no search finds the ball. It lands in, takes its second contact, and the point goes to the hitter.

## Port fix
The port's `predicted_path` used to step `g.flight` with the net on and counted the net touch as a bounce. That
let the receiver hit a net cord after it landed, and gave a different path before the touch.

Now `predicted_path` matches the original:
- It steps net-free and counts court contacts.
- After the first net touch, it adds `path_base`, which is `Flight::predicted_contacts(hit flight, t)` using
  the fill schedule.

Proof: `crates/hst-sim/tests/live.rs` `net_touch_keeps_the_predicted_contacts`. For all 8 recorded net touches
(vsync 14450, 15016, 16039, 19501, 20089, 21197, 24979, 26299, t = 21–41), the recorded re-seeded predictor
holds 2 contacts while the live ball has 0, and `predicted_contacts` gives 2 for each.

Limits:
- All recorded touches are serves; no real rally net cord was recorded. The re-seed code does not depend on
  the shot class.
- The stand-in AI's `intercept` and the landing mark still step with the net on. They are not the original's
  logic (P11).
- On a second net touch the original saves the re-seeded predictor's count, which is already ≥2. The port keeps
  the first count.
