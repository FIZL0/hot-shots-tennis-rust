# B362 — controller button icons in join hints, Button Icons setting

## What changed
- `legend.rs`: two new pad styles, **Switch** (letters by position: B bottom, A right, Y left, X top; L/R, ZL/ZR,
  -/+) and **Generic** (a four-dot diamond with the meant position filled; L1/R1, L2/R2, Select/Start). Auto
  detection adds Nintendo's vendor id (0x057e → Switch). `pad_style` picks the Button Icons setting's style for
  every pad when it isn't Auto. `HST_LEGEND` also takes `switch|generic`.
- New setting **Button Icons** (Settings → Display, `button_icons = auto|ps|xbox|switch|generic` in settings.txt,
  `Settings.9`): Auto / PlayStation / Xbox / Switch / Generic, applied live via `legend::LAYOUT`.
- Join hints: the main screen's empty device tile and the select's open/CPU cards said "Press X / Enter to join" in
  text. They now draw "Press [pad ✕] / [Enter] to join" in the legend's art (`home::press_hint`, `Tex::Button`,
  `legend::Buttons` baked once at startup). The pads shown are each connected pad's style, deduplicated (from
  `note_pads`), then the keyboard's Enter keycap; with no pad plugged in only Enter shows. The line shrinks as one
  to fit the card.

## Checked
- Shots in `context/b362/run1/` (1280×720): home tile in ps/switch/generic/xbox, the select's CPU cards (switch:
  card hints and the legend strip both B / Y / X / L R), Display page with the new row.
- Tests: `button_icons_pick_the_style` (vendor detection, setting override, Switch/Generic glyphs, font has - and +,
  every style's ✕ baked), `presets_and_switches` (setting saved/parsed). The font has no U+2212, so Switch's
  minus is drawn as "-".

## Not verified / not 1:1
- Real Switch / generic pads not tested on hardware (no such pad here): vendor detection and the shown letters are
  by position (Bevy's South = the bottom button), checked only by `HST_LEGEND` shots.
- The in-match "✕ Continue" (result screen) and "✕ Skip" (intro) still use the disc's PS2 ✕ glyph: they are the
  original's art, already an icon, and stay PS-style whatever Button Icons says.
- Front-end is ours to design (PLAN), so no comparison against the original.
