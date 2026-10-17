# B564 — Controllers screen: which controller is which (concept C)

## What changed
- `assign.rs` `foot`: the card's foot (now 38 px) is a chip in the seat's colour (free/COM seats: the seat's light
  well) with a dark square badge — a pad's number over a faint pad outline (body + two grips), `KB` for the keyboard,
  `COM`, `-` when nobody has it — beside the device name (`dev_name`) and its role: 1P controls / Player /
  "OK or Enter joins" (`dev_name(Dev::Open)`, only where an unseated device could join) / Free / Plays itself /
  Another machine / "Yours when you leave" (B437). 1P's held-card arrows and ± COM stay on the left of the chip.
  The old join prompt (button glyph + "to join") is replaced by the role line.
- `assign_shake.rs` `flash`: any just-pressed button on a seated, readable pad (any key for the keyboard's seat)
  sets `Menu::flash[p]` to 1, fading to 0 over 1 s; the pad gets a 0.2 s half-strength rumble, once per flash (a
  press mid-flash restarts the glow without buzzing again, so menu navigation doesn't keep it rumbling). It only
  observes: the press still does its normal thing; the flash moves no focus (test checks `sel`/`held`).
- `assign.rs` `glow`/`bubble`: three gold rings fading out round the card, and over its top a dark bubble with a
  gold rim and a small gold tail: "Controller N pressed - this is you" (or "Keyboard pressed - …"). Both move with
  the P21a shake. The description bar keeps the full 1P help; it gains "any of its buttons flashes it".
- `HST_FLASH=<seat>` draws a card mid-flash for `--shot`s.

## Verified
- Tests: `assign_shake::a_press_flashes_its_card_and_rumbles_its_pad` (seated pad / unseated pad / keyboard, fade,
  single rumble, restart, only on Assign, no focus change), `assign::badges`, `four_cards_fit_the_pause_pool` with
  all four flashing.
- Shots: `HST_MENU=assign HST_FLASH=0 tools/shot.sh` at 1280×720 and 720×1280 (`context/b564/run1/`), checked by
  eye: chips, badges and bubble lay out at both aspects.

## Not verified / not 1:1
- A pad's numbered badge with its outline and the real rumble weren't seen on screen: `--shot` runs read no gamepad
  (P62), so only the KB / COM / - badges were shot; the pad badge is covered by the `badges` test only, the rumble by
  the message test (whether a given pad/driver honours Bevy's rumble request is untested here).
- The concept board (claude.ai artifact) couldn't be opened from this unattended run; the look follows the task
  line's description, not the board pixel for pixel. Front-end, so ours to design (PLAN.md).
