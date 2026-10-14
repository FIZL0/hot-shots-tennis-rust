# 1 — the startled creatures' weight is their alpha (FINAL)

## What the creatures are
- Type 31 (trg_SA-05) is the crocodile. It is hidden at rest (`on` 0, weight 0). There are two instances, one behind
  each baseline, and each has a path running right to left along its end. Row: mode 5, speed 0.06, steer 0.1, near
  [0.5, 0.5], sound 34, no rearm.
- Type 32 (trg_BA-02) is the spiky reptile. It is visible at rest (weight 1). Its paths run off into the jungle.

## The walk matches the original
- A startled crocodile rears up at home and starts moving at frame 35. It walks the length of its baseline ("the
  other side of the court" in the report) and goes inactive at its path's end. The original does the same, on the
  same trajectory (original tick 38 at (5.53, 18.68), tick 339 at (−11.69, 13.02), inactive about tick 350).
- Type 32 was startled at home in both games by holding an idle P1 next to it for 6 ticks
  (`context/b280/nat.py`). Both walk out of frame and go inactive at tick 289 in the original, 288 in the port.
  Screenshots agree.
- Resets match: `done` stays set until game messages 6/0x1b, the serve set-up leaves a finished creature inactive,
  and the instant replay restores the snapshot.

## What was different: the weight
- The original's trigger tick writes each new +0x274 value (the port's `speed`) into the model's weight (model
  object → +0xc → +0x5c) for every type. On the first change away from 1.0 it also calls a model vfunc first. The
  startle reset sets the weight to 0 (types 0, 1, 5, 31, 39) or 1 (type 32).
- Poking the weight in the original while type 32 sat near its path end showed what it does: 0.53 drew the model
  half see-through, 0.15 nearly invisible. **The weight is the model's alpha.** So in the original the crocodile is
  unseen while it rears up at home, fades in over the first 0.5 m of its walk, and fades out over the last 0.5 m.
- The port drew it fully solid for the whole time it was active and then popped it out. That is the closest thing
  to "it doesn't disappear" that could be found.

## Fix
- `hst-sim/src/npc.rs` `step_near`: the weight takes each new `speed`. Unit test `startled_weight_fades`: the
  weight stays 0 for 30+ ticks, rises strictly to 1, falls over the last ticks and ends below 0.1.
- `hst/src/play/npcs.rs`: types 31/32 fade by their weight. Fading now works on the rigs' `GsMaterial` draws:
  material colour alpha × a, TFX MODULATE, Normal blend, and hidden outright at 0. The old fade looked only for
  `StandardMaterial`s, which a rig no longer has once `shade.rs` gives it GS draws. So **the passing ball's (type 15)
  fade never worked either**, and it now fades too.
- `hst-sim/tests/npc.rs`: the two trigger-recording checks take the port's weight, because the recording holds only
  the object (0x300 bytes) and not the model it points to.
- Port shots with the far crocodile startled and its weight forced to 0 / 0.4 / 1: hidden / see-through / solid
  (`context/b280/port6/crop2.png`).

## Not verified / not 1:1
- The user's exact symptom was not reproduced as a difference in the sim: the original also walks the crocodile
  across after a startle. What was wrong was how it was drawn (full alpha during the hidden rear-up, no fade-out).
  If "the other side" meant the other end's instance, that one is a separate creature in the original too.
- Types 0, 1, 5 and 39 (other courts) also have their weight written in the original, but they are not faded here.
  The alpha meaning was only checked for 31/32 (`ponytail:` comment in npcs.rs).
- The model vfunc on the first change away from 1.0 (a mode switch) is not modelled. It matters for type 32 at rest,
  where poking the weight had no visible effect before that switch. The port's type 32 sits at weight 1, so it looks
  the same.
- Type 32's draw-time camera rule (+0x71: it counts down and draws only when it is not behind or very near the
  camera) is not ported. It is unrelated to this bug.
- Fade checked by eye only (rendering rule). At weight 0 the model is hidden rather than drawn transparent with its Z
  write. The GS's Z behaviour there was not looked at.
- The weight isn't in the trigger recordings (it is off the object), so its values are checked by the decompile, a
  poke and the unit test, not against a capture.
