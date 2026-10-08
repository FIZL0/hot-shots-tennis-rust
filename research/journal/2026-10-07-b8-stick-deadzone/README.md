# B8 — stick deadzone as the original (FINAL)

## What the original does
Two layers, no radial deadzone anywhere:
1. Pad driver (pad update → per-slot decode): every stick axis byte, both sticks, 0x51..=0xae → 0x80 (digital
   mode 0x41: all 0x80). Already noted in the P0 journal; confirmed in the decode function.
2. Per-player pad read (the function P7 calls "pad read"): axis i = b − (b ≥ 0x81) − 0x7f.
   - Held directions (bits beside the d-pad's, with the same press/held/released 2-bit states): per axis
     |i| ≥ 48. Menus read these: on the pause screen (slot 3, Start) a full stick down/up moves the hand like
     the d-pad (screenshots, PCSX2 copy 5).
   - Left and right stick vectors (+0x60, +0x70): the max-axis dead square 48/127 and rescale (`pad_dir`).
   - Running and aim both use the human direction function's output (`pad_dir`: dead square, ×1.2, clip,
     camera flip): the serve aim gets that vector directly; the rally aim gets it through the play state
     (replaced by the auto-approach direction in some volleys, see P5a).

## Port
- `hst_sim::player::pad_deadzone` (driver bytes), `pad_held` (menu direction); test `player::tests::pad_bytes`.
- play.rs: `deadzone()` (0.2 radial) removed; `pad_stick` puts both sticks through the driver's byte deadzone
  (`stick_byte` and back, lossless for `pad_run`). Shot and serve aim now use `pad_run` (the original's vector)
  instead of the camera-rotated raw stick (`screen()`, removed).
- menu.rs: up/down from `pad_held` (|48| of 127 ≈ 0.378) instead of 0.5.
- Right stick: only the port's free camera uses it; it gets the driver's byte deadzone, nothing else (the
  original has no free camera).

## Not done
- Follow-through/whiff break-off still test the (driver-deadzoned) stick ≠ 0; which vector the original tests
  there wasn't checked.
- The d-pad merges into the stick (pre-existing `pad_run` ponytail), so menu/aim d-pad paths go through it.
- `volleys_launch_like_the_game` fails before and after this change (see B7's journal).
