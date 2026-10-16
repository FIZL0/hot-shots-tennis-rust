# B291 — Controls page: Controller / Keyboard & Mouse tabs

The page already had the menu's row/panel look (B471); this task adds the two tabs and the pad's button icons.

## What changed
- **Tabs** on the list's strip after "Controls", between L2 / R2 (keyboard 1 / 3), the same `look::tabs` strip row as the
  Mods page's kinds; L2 / R2 or a tap switches. The page opens on the tab of the device last used (a phone: Controller).
  The strip's "N actions" count is gone (the tabs take its place).
- **Controller tab**: each row's bindings as the pad's own art (`legend::pad_style_shown`: the last pad used, else the
  first connected, else the Button Icons setting, PlayStation on Auto) on a dark pill: face buttons, L1/R1/L2/R2
  (LB/RT…, L/ZR…), a single lit d-pad arm, L3/R3, Select/Start/home. The fixed sticks are shown too: the left stick pushed that
  way beside Up/Down/Left/Right, the right stick flicked beside the emotes. All baked once per style in
  `hst-play/src/play/legend.rs` (`baked`: page glyphs, then one per `controls::pad_buttons()`, then 8 stick arrows;
  `button_glyph`, `stick_glyph`). Side panel: the bindings as words ("Cross", "D-pad up", "Left stick") and a note that the
  stick can't be rebound.
- **Keyboard & Mouse tab**: key caps in a white pill (`key_name`: W, Left, Num 4, L Shift, Space); a note on what the mouse
  and the menus' fixed keys do.
- **Rebinding** is per tab: the Keyboard & Mouse tab accepts only a key, the Controller tab only a button from any connected
  pad. Before, both were accepted at once. Escape or a tap cancels. The legend says "Press the new key" / "Press the new button".
- `HST_MENU=controls-keys` / `controls-bind` for `--shot`s.

## Checked
`--shot`s in context/b291/run2/: 16:9 PS / Xbox (waiting row) / keyboard, 600×1000 portrait (Switch art), 1400×450 very wide.
Tests: `options::tests::controls_tabs`, `legend::tests::controls_page_icons`, the main menu suite.

## Not verified / not 1:1
- Front-end, ours to design (PLAN.md): there's nothing in the original to compare.
- "Per seat": bindings are still one `controls.txt` shared by every player and pad (the footnote says so). Any seated pad
  can rebind. Separate bindings for each seat would change the match's input reading (`read_input` in play.rs, which other
  agents are editing), so it isn't part of this task.
- Not tried with a physical pad pressing a button to rebind (no pad on this machine). The code path is the old one plus the tab filter.
