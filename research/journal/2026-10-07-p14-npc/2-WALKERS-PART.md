# P14b — walking spectators' animation

Recorder: `tools/record_npc.py <slot> <frames> <out>` (walkers found by vtable 0x1d1de0 in the slot's .p2s RAM;
per frame the globals, the shared MT, gallery manager +0x680..+0x900 / +0x1b60..+0x1b80, judge, and each walker's
object, animation controller and animation header). It stops when the match ends: the first version spun forever
after slot 5's "game set match" (its walker regions never read the same twice) and held the PCSX2 lock until
killed. `context/fixtures/npc_s05.bin`: slot 5 (court 10, doubles bots), 2268 frames from vsync 7527 to the
match's last point at 9778.

## Animation controller (0x1404c0 set / 0x1407d0 advance)
ctrl +0x34 speed, +0x38 frame shown, +0x3c next, +0x40 step (= speed), +0x78 loops, *(+0x24)+0x2c length.
set(t): next = t, clamped to [0, len] (looping: wrapped by repeated sub/add of len), frame = next.
advance: set(next); next += step. Setting an animation (0x39e3a0): set(0), speed 2.0 for anims 0/1 with
four players (else 1.0), loops for 3/5, +0xcc = (anim != 4); anim 4 advances once at once.

## Walker tick (0x39f160 → 0x39e8b0)
- Raining (court +0x135 = 2/3) or disabled (+0x301): nothing.
- Stagger counter +0xd0 (≥ 0 after a new point): equal to slot → anim 0 at
  lerp(slot·w, (slot+1)·w, r·2⁻³²), w = len0/6 (0x39f670; the anim argument is the caller's 0 in $a1); then
  +1, −1 once past the slot. So after a new point walker k restarts on tick k.
- Mode 1 (point message): singles → anim 5. Doubles with 0x4230b8 ≠ 2: manager +0x8a5+slot set → anim 5 (cheer
  loop, plus a manager stand record), else anim 2 (the argument Ghidra lost: `li $a1, 2` in the branch delay of
  the first test); then frame lerp(5·slot, 5·slot+5, r·2⁻³²). Mode → 2. (0x4230b8 = 2 picks by the judge /
  favoured side: never 2 in the recordings, not ported.)
- Anims 1..4 at their end → anim 0. Anim 0 at its end → MT draw: ((r>>16)&0x7fff)%100+1 < 21 → anim 1, else
  set(0).
- Advance: four players and anim < 2 → walkers 0–1 on even, 2–5 on odd values of manager +0x1b74; else when
  +0xcc.
- u32 → float: `cvt.s.w` (halved, or'd and doubled when the top bit is set) = chop; `ps2::utof`.

## Timing details found in the recording
- Manager +0x1b74 counts after the walkers' ticks and restarts (0) at a new point, also after them; the sample at
  a new-point frame is read before that frame's count (the frame is long: the game stood still for 3 frames).
- The MT is reseeded at a new point (index 25 → 1, words unrelated to a regeneration): the test takes those
  ticks' draws from the new words.

## Proof
`walkers_animate_like_the_game` (tests/npc.rs): every walker tick from the recorded state of the tick before
reproduces anim, frame, next, speed, mode and counter bit for bit; draws must be consecutive outputs of the game's
MT within that tick. Court 10: 9008 walker ticks, 67 with draws, 8 reactions. Courts 1, 2, 4: pending capture
(PCSX2 busy with another agent's recording).

## Not here
Movement (dodge, avoidance, collision, ground height, facing while reacting, reset facing) → P14e: no recording
has a walker moving. The app's walkers are capsules: nothing to animate until their models are drawn.
