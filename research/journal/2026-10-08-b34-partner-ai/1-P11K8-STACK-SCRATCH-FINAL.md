# P11k8: NET gap in ai_human_vpad_b.bin (FINAL)

## The gap

ai_human_vpad_b.bin, NET (tag 2), v9760–9764, substate 0, no find. In these calls:
- `after` (0x362090) is true with the recorded t (fixture +0x730, z < 0, side +1).
- The partner's record (slot 0, the human, kind 0, rewritten every frame) has n ≥ 0.
- So the port follows: it repicks Other, sets follow and makes the call.
- The game instead dives and chase-walks.

Either forcing after() false, or forcing n < 0, made the replay bit-exact.

## Checked

- The 0x35e560 record read, the after/landing_short/copy-path decomps and their disassembly all match the port.
  - The sign compare in 0x362090 is `c.eq.s` of two ±1 floats.
- The searches before 0x362090 (0x361760, 0x3613e0, 0x360910) write the scratch only when they find something.
- The NET prologue saves registers at sp+0x20..0xb0, below the scratch.
- `HST_PROBE=1` (tools/record_ai_rally.py) logs two things:
  - what 0x362090 reads, called from NET (ra 0x3d0c9c);
  - the mate record 0x35e560 returns right after (ra 0x3d0cc4).
- `research/p11k8_probe.py` compares the logged reads with the entry records.
- Captures, 4000 frames each, tag 2:
  - p5 (slot 4);
  - m3a–d and m4a–d (slots 3/4, P1 right-handed Carol as saved).
- All of them replay bit-exact. They include 4 follow cases with mate n ≥ 0 (kind-2 records).
- Two reads differ from the entry record: m3c v6843 and m3d v7900. Both read 0, and in both a vsync ticked between the entry hook and the read.

## Cause

The scratch is the caller's stack below sp. The entry stub took it and then spent a long time copying the path. A vsync interrupt in that window writes below sp (seen as zeros). The game, under the hook, read 0:
- after() is false, so there is no follow;
- this is the same as the bit-exact forced case.

The recorder now takes the quad again as the stub's last step (before the jump into the routine). On f3a/f4a (slots 3/4) every read matches the entry record, 117 of 117.

The test overrides the 5 calls in ai_human_vpad_b.bin with t = 0. That fixture can't be re-recorded (pad-driven).

## Not verified / not 1:1

- The value the game read on ai_human_vpad_b.bin v9760–9764: not logged then. 0 is inferred from the two clobbers seen since, which both read 0. Any z ≥ 0 gives the same result.
- The clobber needs the interrupt to land while the dispatcher sits at that stack depth. Under the hook that is the long stub; unhooked it is a short window. The port's `stash` is the previous call's scratch, so a clobber in the unhooked game is not modelled. It's timing-dependent and can't be derived from game state.
- The fixed recorder hasn't caught a clobber yet (f3a/f4a had none), so that it records the clobbered value is checked only by the code order.
- Who writes the zeros (the kernel's interrupt entry, presumably) was not traced.
