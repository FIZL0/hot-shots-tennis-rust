# B451: umpire picker over the players

OK on the confirm screen's Umpire card (or the lobby's Umpire pill) now opens a panel over the players' area,
not the old full-screen Select Umpire. The panel is the same as B450's court picker, which was being written in
parallel (slot s3, `court_picker.rs`); this one copies its shape, constants and look (`umpire_picker.rs`):

- row 1 is the disc's five with Random first; row 2 holds the umpire mods and shows only when one is installed.
  Each row scrolls on its own ("< a-b of N >", a scroll bar, L1/R1 a page) and keeps its place across ↑↓.
- a card is the round face (a mod's own head, B400; the disc's `Confirmation_00` face; Random's "?"), the name,
  and where the umpire comes from ("Hot Shots Tennis" / the mod's game / "Picked at the start").
- a mod that ships any of Preview Voice's calls (`voice/<prog>_<key>.wav`, checked once in `umpire_mods::load`,
  `Menu::umpire_voices`) carries a "△ voice" tag drawn with the legend's own △ art. △ plays a call only for a disc
  umpire or such a mod (a mod with no calls used to play its donor's). The legend reads ←→ Choose · ↑↓ Row ·
  L1 R1 Scroll · △ Hear voice · ✕ OK · ○ Cancel; Row, Scroll and Hear voice appear only when they do something
  (this covers B431).
- whatever the panel covers is dropped from the draw list. Doubles confirm plus the panel came to about 4000
  quads, over the menu's 3200-quad pool, so the second row got cut off. Test: `picker_fits_the_pool`.

Merge notes (kept apart from B450's hunks on purpose): the step and draw go through new match arms
(`Screen::Confirm … && sel == 4`) ahead of the old ones. The old picking branch's umpire half and
`umpire_screen` / `strip*` / `name_card` can't be reached for the umpire any more. Once B450 has landed (its court
picker also stops using them), the master can delete them, and can merge `area`/`view`/the row stepping of the two
pickers into one module.

Shots (ours, `context/b451/`): 16:9 with eight Get a Grip umpire mods plus one copy with no voice, 4:3, a 600×900
portrait, and the disc-only row.

## Not verified / not 1:1
- The board ('Concept v4: Umpire pressed') couldn't be opened: the claude.ai artifact needs a login. The layout
  follows the task line and B450's panel, not the board itself.
- The lobby-opened picker is drawn over the lobby but checked only by a unit test (it draws and fits the pool, and
  the existing lobby test opens it, puts it back and returns). No screenshot: there's no `HST_MENU` for it.
- The confirm screen's help line under the panel still reads "OK: choose the umpire · Back: …" (confirm.rs belongs
  to B449, which is rebuilding it and drops the help line for the legend).
- Front-end only, so no original to match 1:1 (PLAN: menus are ours).
