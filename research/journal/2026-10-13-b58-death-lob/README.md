# B58 — death lobs (framed mis-hit: skull pop-up + clank)

## What it is
The "death lob" is the framed mis-hit (player flag `framed`). It is rolled only on a grade-4 (SLOW) contact at
height ≤ 0.4, on kind 0/1/2 ground strokes, volleys and dives, and it launches a lob (kind 3) to a random spot. The
launch was already ported and proven bit-exact by P3a (`swing::mis_hit` / `swing::wild_aim`, 6 framed launches in
`tests/timing.rs`; journal `2026-10-07-p3-timing-grades`). The sound of the ball in flight for a framed hit (mode 1)
was already in `sound.rs`. Missing were the skull pop-up and the hit's clank.

## Rule (decompile + live capture)
- On the hit event, the effects manager copies the hit's framed flag.
- The pop-up manager reveals the pending timing balloon as usual, except when the framed flag is set at that
  moment. Then the balloon is moved straight to its last stage (never visible) and the manager remembers the player.
- In the next frame's player loop, that remembered player gets a kind-7 pop-up using sprite 12
  (`fukidasi_dokuro.tm2`, the skull). It uses the "!" fade timing (3 in, 30 held, 3 out). The manager has a single
  slot for this, reset after the spawn.
- There is no human/CPU check. While a "!" is up, bad-timing balloons are removed when created, so no skull appears
  then. The serve-setup message clears pop-up kinds 1/6/3 only, so a skull survives it.
- The clank: `hit_sounds` plays slot 9 program 0 key 4 at 0x80. Slot 9 is the system bank `SND/SE/SYS/SYS_SE00.XB`
  (as found for B42). The port had skipped slot 9, so the clank was silent.

Capture: `research/b58_skull_rec.py 5 3000 context/b58/run1` (slot 5, bots).
- v8351: player 2 is framed and the effects manager's flag is set; the turtle balloon has been pending since v8350.
- v8352: the manager remembers player 2.
- v8353: the skull spawns (kind 7, sprite 12) and fades in (43/86/128). It holds for 30, fades out, and is gone at
  v8389.

Screenshot of the original: `context/b58/run1/skull.png`. Skull of the port, forced by a local debug hack (not
committed): `context/b58/port/skull.png`. Same art, drawn over the head.

## Changes
- `hst-sim/src/surprise.rs`: `Popup::Skull` (texture, "!" frames), test `skull_frames`.
- `hst/src/play.rs`: `Player.framed` is set in `strike`. `play_sounds` plays slot 9 from the system bank.
- `hst/src/play/intro.rs`: `system_bank` loads `SYS_SE00` when the intro didn't (`SystemBank` is now `pub(super)`).
- `hst/src/play/surprise.rs`: a framed hit drops the player's balloon, and the skull spawns the next tick. The skull
  also survives serve setup. The view count is 12.

## Not verified / not 1:1
- Frame timing of the port's skull vs the capture: not compared tick for tick. No replay can be driven to a framed
  hit in the port, and the capture's samples are partly torn. It uses the same fade code and frames as the "!"
  (test `skull_frames`), and the 1-tick delay follows the decompile.
- The skull in a real port match: not seen. A 5-minute CPU match had no framed hit. It was checked only by forcing
  it in place of the "!" (debug hack, reverted).
- The clank: not checked against an SPU recording. It plays the slot/program/key the decompile names, and was
  not listened to in a run (unattended).
- Single pending slot: if two players were framed in the same tick, only the last would get a skull, as in the
  original's one slot. This is ported as is but never observed.
- The practice-mode branch of the pop-up manager: not ported. Practice mode isn't in the port.
