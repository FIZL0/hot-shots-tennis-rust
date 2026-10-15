# B412 — High graphics preset

- `Graphics::HIGH` (graphics.rs): 100 % scale, High shadows, 4x MSAA, uncapped, no vsync, Fewer Shadows off,
  Skip Off-Screen (cull) off, Mesh 100 %. First in `PRESETS` (High, Default, Steam Deck, Low, Phone); Default is
  unchanged (the same picture with Fewer Shadows and Skip Off-Screen on).
- The speed-up switches are now part of a preset: `keep()` only carries the window mode and Show FPS through a
  preset change, so `preset()` reads Custom when Fewer Shadows, Skip Off-Screen or Mesh differs from every preset.
  The options page's "Set by Preset" note covers Mesh, Fewer Shadows and Skip Off-Screen too; the Preset help text
  names High.
- Stepping Preset from Custom goes to the list's first entry, which is now High (was Default).
- Existing settings.txt files that had a speed-up switched away from its default now show Preset = Custom.
- Tests: `graphics::tests::text_and_flags_round_trip` (High's values, flags, name order, keep), `options::tests::
  presets_and_switches` (High ↔ Default via the switches, Mesh makes Custom).

## Not verified / not 1:1
- Sway LOD isn't part of a preset: it lives in the menu's settings flags, not `Graphics`, and B411 may remove it.
- No screenshot of the options page taken: text and list changes only, no layout change. Front-end, so no 1:1 check.
