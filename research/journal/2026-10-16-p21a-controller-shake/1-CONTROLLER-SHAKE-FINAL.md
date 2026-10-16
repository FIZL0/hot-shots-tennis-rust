# P21a — Controller assignment: a pad's right stick shakes its seat

The assignment screen itself (P1 picks which device drives which player) already landed with B40/B238 (menu) and
B254 (pause menu). What was left of P21a was the original's way of telling controllers apart: the selector of the
controller whose right stick moves shakes.

- `main_menu/assign_shake.rs`: on `Screen::Assign` (menu and pause overlay, same screen), every readable pad whose
  right stick is past half way keeps its seat's card shaking; let go, the swing dies out over 0.3 s. Swing ±6 units
  (640×448), 14 per second. The offset goes on `Menu::shake`, written only when it changes; the Assign layout shifts
  that card's quads (box, label, device name, ✕, ± COM) by it. Hit rects stay put.
- The help line ends with "A controller's right stick shakes its seat."
- Unseated pads have no card, the keyboard has no stick: nothing shakes for them. Other screens: the right stick
  stays B460/B463's emotes.
- `HST_SHAKE=<seat>` draws that card mid-swing for `--shot`s.

Test: `assign_shake::tests::a_pads_right_stick_shakes_its_card` (own card only, threshold, settling, Assign only).
Shots: `HST_MENU=assign [HST_SHAKE=0] tools/shot.sh context/p21a/assign{,_shake}.png`, checked by eye (1P card
offset, the rest in place).

## Not verified / not 1:1
- The original's shake (its amplitude, rate, duration, whether it shakes while held or once per move) wasn't looked
  at in PCSX2: the front-end is ours (PLAN.md, user 2026-10-12), so the numbers are our own choice.
- Not driven with a physical pad (none on this machine run); the stick path is unit-tested on a `Gamepad` component,
  the drawing checked with `HST_SHAKE`.
- Only checked at 16:9; the offset is in the layout's 640×448 units, so it scales with the screen like the cards.
