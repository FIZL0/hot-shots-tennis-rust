# B396 — Lobby v3 (FINAL)

Rebuilt the online lobby from the approved board *Proposal: Lobby v3 (online, doubles, host)*
(singles = same board with players=2). Board copies used: `context/b396/` (not in git).

## What changed
- `crates/hst/src/play/main_menu/online/lobby.rs` rewritten. Layout: equal seat cards grouped
  TEAM 1 · VS · TEAM 2 (singles: one card each side), side panel (180 at 640 wide = 300 at 1280) with court picture +
  name, Mode / Sets / Games / Mods pills two by two, Umpire full width, invite box (QR, JOIN CODE, Copy link,
  Public); bottom bar Leave lobby, Chat, help line (focused seat's name · style, then its controls), big Start ▶
  under the side panel with "n of N ready · <who> is still picking" (guest: Ready / Not ready).
  Each card: seat tag, YOU + white focus ring on this machine's seats, status chip (Ready ✓ / Picking… dashed /
  COM · rank / Open), host ✕ remove, 3D preview with the mod stand-in note over its foot, name, machine · ping
  line (ping gold above 150 ms), costume + hand chips, play style, pink stats card; COM/open seats dashed.
- Dropped: the settings row, the separate Characters / Copy link / Public buttons, B370's wide own-player column.
- Any aspect ratio, uniform scale: wide → side panel at the right; portrait → side panel under the teams
  (court left, pills + invite right); doubles in a tall window stack TEAM 1 / VS / TEAM 2 as rows.
  Test `lays_out_at_any_aspect` checks 4:3, 16:9, 21:9, 1:1, 3:4, 9:16, 0.4.
- `online.rs`: item list is now seats (+ ✕ after each removable seat), the side panel items, then the bar buttons;
  arrows walk cards → ✕ → side panel → Start; ←→ change pills; on your own card L1/R1 = costume, Select = hand,
  OK = pick again (unreadies you). Umpire is a lobby setting (stock umpires + umpire mods).
  Guest: no ✕, no ‹ ›.
- `chat.rs`: `ASK` flag so the Chat button opens chat (typed box from keyboard/mouse, phrases from a pad).
- `legend.rs`, `pointer.rs`, `main_menu.rs`: legend for the own card, test updates.

## Checked
- `tools/check.sh -p hst`: 393 passed.
- Screenshots (`HST_MENU=lobby|lobby-guest|lobby-bot` via `tools/shot.sh`) at 16:9, 4:3, 21:9 and 9:16:
  `context/b396/run1/`. Compared by eye with the board.

## Not verified / not 1:1
- Board's "△ name" control is not in the lobby (names are set elsewhere); help line lists only real controls.
- Start sub-line names the player ("ACE-42 is still picking"), not the character — the character isn't chosen yet.
- ✕ is drawn as "X" and ‹ › as "< >": the menu font has no such glyphs. Court card says "OK: change" for the host
  instead of "✕ change" (OK is the button that changes it).
- COM chip shows the CPU rank name ("COM · Hero") rather than "Lv N": ranks are what the rest of the menus show.
- Chat button from a pad opens the phrase list (like R2), from keyboard the typed box — board doesn't say.
- Doubles in a tall window stack the teams as rows: the board only shows landscape.
- Hand cursor drawn over the focused card's corner in addition to the board's focus ring (menu-wide convention).
- No PCSX2 comparison: front-end is ours to design (PLAN.md).
- B392/B393 (listed as "build after") were still open in other slots; built on current main, so their merge may
  touch `online.rs` / `lobby.rs`.
