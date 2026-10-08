# B27d: the rally sub-state 0 decision tree is P11k2–k3's work (blocked)

B27d asks for the doubles BASE/NET rally routines (3d1320/3d02c0), their singles twins, the path gate 35f0d0, the contact
searches, the +0x70 lock and the spot helpers, ported and wired into `play.rs` in place of `intercept`. That is the
same work as P11k1 (receive + spot helpers 361af0/3621d0/361e70/361eb0, path copy 35f0d0), P11k2 (NET/BASE with
3613e0, 3d23f0, 3d5bb0, 3d5dd0, 35e510/35e560) and P11k3 (dispatcher 3cebf0, wiring into `bot`), with P11l the singles
part. Task/P11 already landed the contact searches (P11j: 360150, 360910, 35bc50, `tests/ai_search.rs`), and while
B27d ran its agent was writing `crates/hst-sim/src/rally.rs` with `tests/ai_rally.rs` and `tools/record_ai_rally.py`.
A second port here would make a clashing rally module and double the work, so B27d waits on those tasks.

## What to keep from the reading
- The decompile hides stack arguments; the disassembly shows them. 360150 from the rally routines: a0 ai, a1 −1,
  a2 1, a3 &target, t0 &ball, t1 &frames, t2 &ball frames, t3 1 (search all), stack: the +0x24c byte. 360910: a2 level,
  a3 minimum bounces, t0..t3 the pointers, stack: 1 (search all), +0x24c (zeroes the width), +0x24d (zeroes the
  height: the smash-only start).
- Disassembling the overlay: wrap the raw segment in a minimal ELF (EM_MIPS, one .text at 0x322d00) and run
  `llvm-objdump -d --triple=mipsel --mcpu=mips3`. Host objdump has no MIPS and llvm-objcopy rejects elf32-littlemips.
  P11j's `research/dis.sh` does the same.
- The path copy 35f0d0 is cached once per frame (0x427108/0x427110) and clears the 0xb4 search marks at 0x427050 on
  each call; the entries come from the marker gm+0xa4 (+0x50 pointer, +0x54 count, +0x58 index).

## What unblocks it
Once P11k2–k3 merge, B27d is only the check: run `player.rs ai_serve_stick` over serve_ai.bin with the ported tree
driving the stand-in's sub-state 0. Every move/no-move and sub-state 1 frame must match: vsync 8800 locks and moves at
once, 10339 moves by a spot helper, 10340 goes to sub-state 1. If P11k3's own check already covers serve_ai.bin's
sub-state 0 exits, close B27d as done by P11k3.
