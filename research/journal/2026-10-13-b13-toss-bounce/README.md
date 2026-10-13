# B13 — no bounce effects for a toss that drops before the serve

**Bug.** A toss left untouched bounces once before `serve_turn` re-holds it. `start_effects` fed that bounce to
`effects::BallBounce`, so a mark, the ring and a puff appeared at the server's feet.

**The original.** The bounce object only builds a mark, ring or puff from the ball's contact records. Its new-bounce
branch is skipped while the record pointer (ball +0x8f8) is null. The ball step adds a record only when the game phase
(gm+0x55) is 3 or more (rally / point over), or its sub byte gm+0x56 is 4, or the step's own flag allows it. During
the serve (phase 2) a bounce counts but leaves no record.

**Capture.** Run `context/b13/toss_bounce.py` on copy 4: slot 3, vpad presses □ (1.5 s) then ✕ (4.65 s), and the toss drops untouched. The ball's
bounces/contacts went 0 → 1 (frame 408) → 2 (427) in phase 2. The record pointer stayed null and marks, ring, seen
and puffs all stayed 0.

**Port.** A new function, `play.rs` `bounce_recorded`, says whether a bounce now leaves a record:
- in `Phase::Serve`, only once the swing is pressed (`serving.swing` or `serving.whiffed`), since the original's
  rally phase starts the tick after the press;
- in every other phase, always.

`BallBounce::frame` takes this as `recorded` and pushes no contact without it. Marks still age, puffs still run.

Test: `play/serve_check.rs` `untouched_toss_leaves_no_bounce_effects`. An untouched toss's first bounce is not
recorded; a whiffed toss's bounce (E2_late script) is.

## Not verified / not 1:1
- **Whiffed toss's effects**: in the port they show, by the phase rule (the original's phase is 3 at that bounce,
  per B29). No capture of the bounce object at a whiff was taken.
- **gm+0x56 == 4 and the step's flag argument**: these also allow a record during the serve. Their meaning wasn't
  traced, and nothing in the port maps to them. The capture shows neither applies to a normal serve.
- **Screenshot**: none taken. The change only removes effects; the capture's counters show the original has none.
