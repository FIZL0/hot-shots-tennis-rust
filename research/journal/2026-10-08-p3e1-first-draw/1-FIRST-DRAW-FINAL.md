# P3e1 — the court draw at a point's first tick

## Result

The "unexplained" draw is walker 0 restarting its idle loop in the reseed's own tick. It was a torn sample, not a
missing port. `court_draws_like_the_game` now replays that tick (walkers from npc_s05's 7565 state,
`new_point(stagger)`, one `step` at gallery tick 0). The replay must give npc_s05's 7566 walkers exactly and make
one draw. Then the frame loop runs as before, with no bare draw. Both rng.rs tests pass. No port change: the app
(`play/npcs.rs`) already calls `new_point` and steps the walkers in the same tick.

## How it was found

1. **Lock-step recording.** `research/p3e1_gallery_rec.py 5 80` (HST_LOCKSTEP=1) recorded these from the slot-5
   load: the gallery manager (*0x43b1c0, 0x1bd0 bytes), the umpire (*0x42d6c0), gm, and the court generator.
   - The court generator matches rng_s05 bit for bit at every frame except 7566.
   - In this capture 7566 has index 1. rng_s05 has index 624 there: fresh from the reseed, so rng_s05's
     real-time read landed between the reseed and the walkers' step in the same tick.
   - After that, one draw per frame: 7566→7567, 7567→7568, 7568→7569.
2. **Gallery and umpire ruled out.** Over 7565–7567, neither changes the stand (+0x86c) or the event
   (+0x860/+0x1b68).
   - The 0xe handler turns the cheer off (+0x864 = 0) before the gallery's step, so the pending `next` == 1
     never fires.
   - The umpire's msg 0x17 → 0x39da50 is the decided point's applause (already `Gallery::point`), not the
     new point.
3. **Poke-and-diff.** From slot 5, lock-step to 7566 and save at 7567 twice: once as is (scratch slot 8), and
   once with MT word[index] at 7566 XORed with ~0 (slot 9). Then diff eeMemory.bin.
   - Apart from the MT word, the only data that changes is walker 0x19edb50's animation controller (+0x38/+0x3c
     frame 26.75 vs 24.07) and the bone matrices that follow from it.
   - That walker is slot 1 (vtable 0x1d1de0), so the 7566→7567 draw is walker 1's restart (`random_frame`).
   - By the same stagger, walker 0's restart is the draw inside 7565→7566.
4. **npc_s05 agrees.** Walker k goes anim 5 → 0 at 7566+k−1, with counter 1, 2, … for the others.

## Technique worth reusing

To find who consumes a given draw: in lock-step, flip the generator's next word just before that draw. Then diff
the whole of EE RAM from two save states one frame later. The only differences are the consumer's state.
