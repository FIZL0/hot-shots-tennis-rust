# M2b — match input as `[Pad; 4]` per tick

The match now takes its input from outside: `play/tick_input.rs`.

- `TickPads` holds this tick's and last tick's `[hst_sim::replay::Pad; 4]` (seats 1P..4P, the wire format).
  `control` reads only `TickPads::slot(s)`: stick from the bytes, d-pad bits, shot/serve = a bit set now but not the
  tick before (the original's edge, as `score.rs`' test reads presses).
- Sources push one `[Pad; 4]` per tick before `control` (`tick_input::systems()`): `local` encodes `read_input`'s
  latches in `Pads.slots` and spends them; `Replay(VecDeque<[Pad;4]>)`, when present, replaces it. The network (M2g)
  is a third source pushing the same way.
- Buttons: ✕ 0x2000 topspin, ○ 0x4000 slice, △ 0x1000 lob (the game's face bits), the keyboard's serve-only key on
  □ 0x8000. Sticks: bytes after the driver's deadzone, as `pad_stick` made them.
- Seats fixed at match start: `fix_seats` freezes `Pads::seats` at the first tick (1P + 2P..4P whose pads are in;
  `--pads` already fixes them at startup). `HST_AUTOPLAY` is read once into `Pads::autoplay` when `Pads` is made.

## Verified
- `tick_input::tests::local_pads_round_trip`: every local pad state (stick over a 401-step grid through `pad_stick`,
  each d-pad/shot/serve combination) encodes and decodes to the same f32 bits, d-pad and presses → `control` sees
  exactly what it saw before: offline play unchanged.
- `tick_input::replay_tests::a_recording_drives_the_match`: `round1.bin`'s 1P pads (1200 ticks) drive the app's
  singles match through `Replay` and the same entry: 1P moves on 108 ticks, differs from idle pads, the same pads
  give the same match twice.
- `tools/check.sh` all pass; `--play --singles` shot: match up, 1P seated (context/m2b/play.png).

## Not verified / not 1:1
- Two presses of the same button within one 60 Hz tick: the second is lost (held bits on the wire; before, the latch
  kept one press anyway). Two different shot buttons in one tick: ✕ > ○ > △ wins (before: the last one pressed).
  Both need sub-16 ms presses; not checked in play.
- A pad plugged in mid-match no longer turns that seat human (seats fixed at the first tick, as the task asks and as
  the original's sides).
- The replay test only proves the recording's pads reach 1P; the app's match doesn't start from the recording's save
  state, so it can't be diffed against it (that's P0's harness in `hst-sim`).
- □ carries the keyboard's serve key: a real recording's □ press would serve as a topspin toss. The original's □ in
  a match was not checked.
