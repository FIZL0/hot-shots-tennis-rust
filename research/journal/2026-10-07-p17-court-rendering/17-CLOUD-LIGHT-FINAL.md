# P17s7: the moving clouds' light and alpha

**Result: exact.** On court 10, all 9 clouds in a GS dump match the formula below to the bit (`research/p17s7_cloud_gs.py`).

## The draw (decompile)

- The court manager draws the clouds only in singles (its +0x136 flag). The tick has the same gate.
- The draw copies the `_clo` view into the current view. That view is a copy of the sky view, which is a copy of the
  main view, so the clouds take the **main view's light block**: colour at +0x60, factors A at +0x70 and L at +0x74,
  and the glare flag set.
- In weather 2/4 the draw scales the light colour RGBA by 0.8, in 3/5 by 0.55, then rebuilds the block. The port's
  `cloud_tint` scales the material RGB instead. RGB is the same product either way. The light colour's w is 0, so the
  tint never touches alpha.
- Each cloud sits at camera + cloud position. The fog is (255, 255) (none).
- The tick works out the fade, clamp((R² − r²)·1/(0.19R²)), with R = 2000. It writes the fade to the cloud (+0x14c)
  and multiplies it into the model's +0x5c.
- The model draw multiplies +0x5c by the record's factor and stores the result at record +0x1c. That is the .w of the
  flags qword uploaded to VU1.
- VU1 flag bit 0 (alpha): the highlight is 0, and that .w replaces the ambient alpha term. Bit 1 (unlit): N' = (1,0,0).
  This gives:
  - RGB = ⌊vc·mat·light·(A + L)⌋
  - A = ⌊vc.a·mat.a·fade⌋
- The cloud ctor ORs flags 3 into the model. It rewrites each material's TEST to `(old & …c001) | 0x160a`, which is
  GEQUAL, AREF 0x60, AFAIL FB_ONLY: below 0x60 a pixel writes colour but not Z.

## The check

There is no singles save state. `research/p17s7_capture.py` loads slot 5 (court 10, doubles) and sets the singles
flag. It then holds the 9 clouds in front of the match camera at radii 1500–1990, so the fades run from 1 down to 0.05.
The wind speed is rewritten every frame, so the script re-pokes the positions until the dump lands. It then GS-dumps
with F11 (copy 3's GSDumpSingleFrame was Shift+F8; it is now rebound to F11 like the other copies) and saves EE RAM.

- The light is (0.863, 0.839, 0.839) with A + L = 1.44, giving RGB (159, 154, 154) on every cloud strip.
- The alphas are 128, 128, 115, 91, 72, 52, 26, 20, 6. That is ⌊128·fade⌋ for each cloud, except fade 0.2599 → 26,
  which is cloud01 (mat.a 0.8): ⌊102.4·0.2599⌋.

## The port

- The port already had the right light (the court light, unlit, glare on) and the right alpha (`base.w·fade` through
  the vertex colour, floored). The ponytail "unconfirmed" note is replaced by the confirmation.
- New: `gs::Test::Ge60`/`Lt60`. `clouds` moves mode 25's 0x70 split down to 0x60, as the game's TEST rewrite does.
- Not checked on a dump: the weather 2–5 tint (decompile only; slot 5 is clear weather).
