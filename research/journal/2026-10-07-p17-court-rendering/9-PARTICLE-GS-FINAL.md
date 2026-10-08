# P17n — weather particles' GS state (P17k gap)

## What the game sets

- The weather classes (rain streaks, ground rain, leaves) each put their quads into a batch object.
- The batch's PRIM has IIP and FGE, plus ABE and TME from the texture's flags.
- Each batch takes one texture object from the effect library: 14 textures (`wind00`.., `rain00`, `groundrain`,
  `leaf00..08`), loaded from `hatsuyama/efct/`.
- A texture object keeps an A+D block in the order CLAMP, TEX0, TEX1, MIPTBP1/2, ALPHA, TEST. The texture load
  doesn't change ALPHA or TEST, so they keep the constructor's defaults:
  - ALPHA_1 0x8000000044: (Cs − Cd)·As + Cd.
  - TEST_1 0x5180b:
    - alpha test on, GEQUAL, AREF 0x80, failing pixels write the frame buffer only (FB_ONLY);
    - Z test on, GEQUAL.
  - The library then sets CLAMP to 0 (repeat in both U and V). TEX1 is 0x60 (linear mag and min).
- Checked in slot 5's RAM (from the save state's `eeMemory.bin`): all 14 texture objects have ALPHA 0x8000000044,
  TEST 0x5180b, CLAMP 0, TEX1 0x60 and flags 0xd.

## Port (`gs::Test::{Ge80, Lt80}`, `play/weather/rain.rs`)

- Each particle mesh gets two GsMaterial draws, the same split as court TEST 20..29:
  - A ≥ 0x80 writes colour and Z;
  - A < 0x80 writes colour only.
- Both draws blend normally and are fogged.
- Rain (vertex A 0x40) never reaches 0x80, so it never writes Z. Fully faded-in leaves on opaque texels write Z.
- Checked with `HST_WEATHER=3 HST_AUTOPLAY=1 hst <iso> --play --stage 10 --shot` (`context/shots/p17n/w3.png`):
  the streaks and ripples draw as before. `tools/check.sh -p hst` passes.

## Not done

- The frame's ZBUF mask during the court pass wasn't read. The court's own draws write Z in that pass, so the
  port takes it as on.
- The textures' sampling and wrap (CLAMP repeat, TEX1 linear) aren't changed. `rain00` stays nearest with U
  repeat, as P17i picked from screenshots.
