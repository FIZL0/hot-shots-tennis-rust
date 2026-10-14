# B295 — Lobby / Confirm redesign (after `mockups/Lobby.dc.html`)

## What changed
- **Lobby** (`play/main_menu/online.rs`, title tab now "Lobby"): the title line "name · Doubles · N sets" is the
  Singles / Doubles item; seats are compact rows inside "TEAM 1" / "TEAM 2" groups: seat tag, round face, character
  (or Random / mod name / "Open seat"), player line (name, or "COM #k · level"), status chip Host / Ready / Waiting /
  COM / Open, and a red ✕ on a computer's row (the Remove item). The context help (the old bottom info text) is a
  line under the seats. The side card: court band (grass stripes + court name), QR (or spinner), JOIN CODE /
  PRIVATE CODE, the code, Copy link and Public/Private pills; Characters / Court under it (guest: Characters
  only); Start match (or Ready / Not Ready / Cancel) as the one big yellow button at the foot, Leave lobby smaller
  under it. The mod-ask popup sits on the side card.
- **Confirm** (`play/main_menu.rs`): same frame: mode line "Doubles · 1 set, 4 games", team groups with seat rows
  (owner · style · costume; chip Ready or the COM's rank), the original's VS sprite and flame between the groups,
  side card with court band, surface + bounce/speed meters, umpire; Select Court / Sets / Games / Select Umpire
  rows; big Start at the foot; help line under the seats. Pickers unchanged.
- Layout: `Grid::split()` gives the seats column and the side card: side card at the right (190 wide) in landscape
  and near-square windows, under the seats (≤280 wide, ≤340 high) in tall portrait ones. `flame::draw` takes its
  origin. All rects come from shared functions (`seat_rect`, `confirm_rects`, `lobby_rects`) that pointer hits and
  the drawing both use.
- Tests: new `lobby_and_confirm_fit_the_window` (aspects 0.4…4: column and card in view, apart, above the legend;
  buttons inside, not overlapping); cursor-walk, tap and hover tests updated for the new positions.
  `tools/check.sh -p hst`: 290 passed.

## Shots
`context/b295/` (lobby-bot, lobby-guest, confirm, confirm-random at 1280×720, 960×720, 1680×720, 720×720,
540×960): checked by eye against the mockup; no clipping or overlap after fixing the near-square split and the
chip width.

## Not verified / not 1:1
- Front-end is ours (PLAN.md): compared with the mockup by eye only; no original to match.
- The portrait is the disc's face icon, not the mockup's initial or a costume render.
- The help line replaces the bottom description ticker on these two screens.
- Confirm keeps no separate Back button: the title tab (and ○) go back, as before.
- A remote mod not installed here shows its donor's face.
- The online flow itself (real peers) was not run: shots use the `HST_MENU=lobby*` sample room; behaviour is
  covered by the unit tests. No PCSX2 needed (front-end).
