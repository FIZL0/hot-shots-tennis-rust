# 1 — FINAL

## What changed
- `main_menu/frame.rs`: the shared v4 frame: navy ground, top bar (title, code pill with Copy link / Public, Singles /
  Doubles button with □), the top-down court with the net and VS badge (turned across in a tall window), a plate per
  player on its half (seat-coloured header, model well, name, style · hand, one status chip), the strip (Court thumb,
  Umpire face, Sets, Games, Mods/Handicap), the bottom row (Leave/Back, chat box, big Start with ready count). One
  `layout()` for any aspect ratio, uniformly scaled.
- `main_menu/confirm.rs`: the confirm screen on the frame; arrows move spatially (`pill_step`); ✕ on Court/Umpire opens
  the pickers (unchanged, B450/B451), Sets/Games keep B212's pick mode; ✕ or ○ on a plate goes back to the select;
  □ switches Singles/Doubles where 1P's seats allow it.
- `main_menu/online/lobby.rs` + `online.rs`: the lobby on the frame; host steps Sets/Games/Mods with ✕/←→ or the ‹ ›
  halves, □ switches the mode, START starts; status messages (mod download waits, network status) go in the chat box.
- `chat.rs`: `RECENT` (last two lines) and `BOXED` (the lobby's chat box shows them, no floating lines while boxed).
- `legend.rs`: v4 legends (hints only in the legend strip).
- B221's title-tab "<" no longer drawn on Confirm (it has its own Back).

## Checked
- `tools/check.sh -p hst --bin hst -- main_menu`: 130 passed (new: `arrows_reach_every_row`, `seat_states_and_ready_count`,
  `lays_out_at_any_aspect`; reworked lobby/confirm navigation tests with BFS reachability).
- Shots (`context/b449/run1`, `run2`): confirm and lobby (host with bots, guest, mod wait) at 16:9, 4:3, 21:9-ish and
  tall 720×1000 — no overlaps, plates turn to top/bottom halves in a tall window.

## Not verified / not 1:1
- Font: the board's Baloo 2 is not used; our UI font throughout. VS is italic by leaning each glyph.
- ‹ › drawn as "<" ">" glyphs, ▶ in Start drawn from quads: our font has no such glyphs.
- ←→ now move along the strip/plates instead of changing values (board shows ‹ › on value cells; values change with
  ✕ / taps on the cell's halves). Design choice for spatial navigation; not specified on the board.
- Confirm Sets/Games keep B212's pick mode (✕ then ←→) rather than direct ‹ › stepping, to keep that tested behaviour.
- Handicap shows "Off": the original's handicaps aren't ported yet (B40g1).
- Chat lines in the box: the speaker is gold, not seat-coloured as on the board (chat has no seat per line).
- Status messages (mod download waits, connection status) shown in the chat box: the board has no place for them.
- Court/umpire pickers unchanged (B450/B451 are separate tasks).
- Front-end: no comparison to the original needed (ours, PLAN.md); checked against the boards by eye only.
- Unrelated update.rs test failure: see Merge note.

## Merge note
- The update test failure came from main (`fbaef6ff`); main fixed it in `77c84a10`, cherry-picked here (update.rs only)
  so the wrap-up's tests pass.
- B450 (court picker over the players) landed on main while this ran and also touches `main_menu.rs`, `legend.rs`,
  `online.rs`; expect conflicts there at merge: keep B449's frame-based confirm/lobby draw + navigation and B450's
  picker (it opens over the screens; this task left the pickers alone).
