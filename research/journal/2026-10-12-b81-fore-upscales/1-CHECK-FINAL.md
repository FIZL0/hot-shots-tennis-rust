# B81 — remove upscales of `"upscale": false` mods

- No upscale is running (`/home/ryha/repos/HST-Remastered-slots/s5/tools/pgrep -f upscale_mods` is empty).
- All 24 `fore_pc*` mods in HST-MODS `out/mods` have `"upscale": false` in mod.json. None of them has a
  `textures/upscaled/` folder (checked every mod folder), so there was nothing to delete. They were either removed
  earlier or never made after the skip went in. `tools/upscale_mods.py` skips these mods (`m.get('upscale', True) is False`),
  so a rerun won't bring them back.
- Other mods (getagrip_*, oob_*) still have their upscales, as they should.

## Not verified / not 1:1
none (file housekeeping, nothing from the original game involved).
