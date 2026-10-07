# N3c

- [x] **N3c — Racket hit, bounce and serve sounds.** Hit = slot 0 (court SE) program 6, key 3..8 by grade/type
  (`0x340860`); find the bounce, net, toss and serve calls (`19fa70` callers) and the 3D position → volume/pan
  (`19b9b0`). Wire them into `play.rs` at the same frames. Verify trigger frames against a PINE recording of the
  command ring (write index `0x304fc0`) next to `match_s05.bin`. j: 2026-10-07-n3-audio

Left from N3b (`audio.rs` ponytail notes): stereo keys are two play calls (`0x1b0098`: two sequences with
positional sequence volume L/R, e.g. 106/43 and 43/106); the driver's 48-voice allocation by priority; the velocity
curve (EE `0x2f1920`, voices so far had velocity 127/100 raw); core/master volume and reverb; sequencer ADSR
overrides at key-on (`0x18f950`).
