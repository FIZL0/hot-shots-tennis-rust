# C9 — C2's three arithmetic changes against the original (FINAL)

C2 moved three spots from plain IEEE f32 to EE FPU ops with only two hash traces as proof. Each is now checked
against the game's own code through the function oracle: `crates/hst-oracle/tests/float_steps.rs` (names in
`research/oracle.txt`, section C9). All three C2 changes were right; the oracle found three other mismatches next
to them, fixed here.

## Character speed (`Stats::new`, SPE / 10)

The TParam row parser runs with its text helpers hooked (strtok → "1", strcmp → match, memset/atof stubbed; atoi
returns 1 except the 42nd read, the SPE column, which returns the test value); the speed is read from the record
table (+0x9c). 12 000 SPE values: every −2000..2000, plus random up to ±2^31.

- `cvt.s.w` + `div.s` by 10.0: the C2 division is right.
- **Fixed:** past 2^24 the int-to-float conversion chops on the EE (1040 of the large values were 1 ulp off with
  Rust's `as f32`). `Stats::new` converts with `ps2::chop`. Real SPE values are single digits; a mod's override
  could be anything.

## Shot table lookup (`shot::lookup`, frames)

The lookup runs whole (both court halves, so the VU0 half-turn too) with the table-pointer and axis-bounds helpers
hooked to give the game the port's random table and bounds (9 presets). The truncation after the last madd is
hooked to read its operands, so the test counts the inputs where the pre-C2 IEEE formula would truncate to another
frame count. 12 000 random inputs (half the tables random throughout, half with frames rising by a fixed step per
target plane), then ~1 900 crafted ones: binary search on the target x (f32 bits) for where the frame count turns
over, then targets 1, 2, 4 … 2^23 ulps below it. 41 of the 13 898 inputs truncate differently under IEEE rounding;
all 13 898 match the port. With the old formula put back, exactly those 41 fail.

- Asm: `adda.s $f2(0), a` then `madd.s fz·(f1 − f0)` and `cvt.w.s`: `(0 + f0).madd(fz, f1 − f0)` is right.
- **Fixed (near-net speed correction):** the pull toward the slowest corner weighs `k · (high · near)` (asm: `mul.s
  f0 = high·near`, then `k·f0`); the port (before C2 too) had `(k · high) · near`, so ~0.6% of random lookups near
  the net had speed 1 ulp off. Now `k * (F(high) * near)` (`high`, `near` are plain `f32` from `cosf`, so the `F`
  keeps the product chopped).
- Interpreter: added MMI `madd`/`maddu` (the lookup's index arithmetic uses `maddu`).

## Run direction (`pad_dir`, final `+ 0`)

The pad read runs on player 1's pad in the slot-3 dump with the driver buffer bytes written (buttons u16, the
stick bytes), then the run direction itself; mode/vibration calls stubbed, the camera-side flag hooked. 16 384
inputs: every x byte against the |48| edges, the centre, the ends and random y bytes, with random d-pad/button
bits and phases 0–5; every other input with the camera on the +z side.

- The final `+ 0` (the identity camera matrix turning −0 into +0): right. Without it 2159 of 65 536 differ.
- **Fixed:** the d-pad takes over when both rescaled stick floats are exactly zero, which includes |axis| = 48
  (bytes 0x4f and 0xb0, both outside the driver's 0x51..0xae deadzone, so reachable). The port had `< 48`, so a
  stick at 0x4f with the d-pad held ran nowhere instead of along the d-pad. Now `<= 48`.
- **Fixed:** with the camera on the +z side the game turns the direction by π through the camera matrix: the
  same values negated, but zeros come out +0. `play.rs` negated (−0). New `player::turned` (negate, then `+ 0`),
  used by `pad_run`.

## Not verified / not 1:1

- `player::pad_held` (menus' held direction) still treats |axis| = 48 as pushed; the game's held bits come from the
  same zeroed floats, so it would read 0 there. Front-end only (menus need not be 1:1), left as is.
- The pad read's held-input lock (pad struct +0x3c: a held mask that zeroes a stick axis until it is released) is
  cleared before every oracle call. It isn't part of `pad_dir` and isn't ported; when the game sets it during a
  match wasn't checked.
- The lookup test hooks the axis bounds instead of running the game's bounds helper, so `Bounds::stroke/volley/
  serve/smash` and the sloped low bound aren't covered by it (they have their own recorded-shot tests).
- Elevation/speed bits are compared too and match, but the random tables are wilder than retail ones (any
  elevation/speed per cell); retail tables are only covered by the existing recorded-shot tests.
