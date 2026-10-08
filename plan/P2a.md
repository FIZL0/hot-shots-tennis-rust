# P2a

- [x] **P2a — Sweet spots by timing, not by memory writes.** P2's recorder (`tools/record_aim.py`) gets its sweet-spot
  hits by writing P1's timing offset (+0x3fa0) to 0 during the locked swing (every other aim), and slices/sweet
  incoming shots by writing the opponents' +0x3ee4 / +0x3fa0. Find out why the recorder couldn't get them by timing
  and fix it, so the fixtures are plain play.
- What P2 saw (journal 2026-10-07-p2-aim): P1's swing locked (+0x3ec1 1..4, +0x3ec4 counting down) with no press
  of the recorder's own at that ball, and the offsets came out 5, −6, 11, 12 regardless of when it then pressed. In the
  port the offset is the frames to contact at the lock − 8 (`find_contact`, `SWEET_FRAME`), so a press that locks
  the swing ~8 frames before contact should be sweet.
- Suspects: the recorder's idle ✕ press (sent to skip past points) or an earlier button press staying buffered and
  locking the swing early; or an auto-swing in the original. Check which, against the decomp (what sets +0x3fa0 and
  when the lock happens) and live: drop the stray presses, press at predicted contact − 8 frames, sweep the press
  frame and see the offset follow it.
- If the original really does auto-lock, say so in the journal and check the port's human swing does the same
  (`play.rs` press/find_contact).
- Done when: record_aim.py makes sweet (|offset| < 2) and off-sweet hits by press timing alone, the
  +0x3fa0 / +0x3ee4 writes are gone (the opponents' slices by their own play or by a pad-driven P2 if needed),
  `aim_singles.bin` / `aim_doubles.bin` re-recorded, `human_aims` still bit for bit with a sweet corner in each mode.
  Singles still needs the player-count write (no singles save) unless a singles save with P1 human is made.
