# B341 — paused L1/R1 no longer queues emotes

Cause: `read_input` (play.rs) latches `SlotPad.emote` every display frame, also while the pause menu is up (the
fixed clock is held, so nothing consumed it). The pause step's `spent` cleared the latched shot/serve but not the
emote, so the touch pad's L1/R1 (B271: its left/right flick), used to tab the pause Settings, played on resume.
Keyboard emote keys 1–4 had the same leak.

Fix (`play/menu.rs`, play.rs untouched):
- `spent` is a free function and clears `emote` too; it runs every frame the pause menu (or its Settings) is open,
  and on the frame that closes it.
- `menu::step` now runs `.after(read_input)`, so a press latched in the same frame is always spent before the
  next fixed tick (before, the order of the two was ambiguous).
- Test `menu::tests::paused_presses_are_spent`.

## Not verified / not 1:1
- Not checked on the browser touch pad itself or in a running window: no Wayland compositor reachable from this
  unattended shell (`--shot` run failed with NoCompositor). Schedule ordering checked by reading every `.after`/
  `.before` against the match chain (no cycle possible); the unit test covers the clearing.
- A shoulder still held at the moment the match resumes emotes, as any held flick does (it's a fresh press for
  the match). The original has no emotes, so nothing to match.
