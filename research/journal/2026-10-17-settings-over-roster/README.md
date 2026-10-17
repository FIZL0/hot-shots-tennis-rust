# B558 — Settings from the roster and the online lobby

## What
The legend's new **Settings** key (O on the keyboard, R3 on a pad) opens the options pages (Display, Graphics,
Sound, Controls, Online & Mods) from the character select (local or online) and the online lobby. Back on any tab
returns to the same screen and row, with picks, seats and the connection kept. The pages are only offered when
nobody is typing (name picker, chat) and no prompt (mod download, takeover, lobby question) or carried card is up.

## How (`crates/hst/src/play/main_menu/over.rs`)
- The pause menu's embedding (`play/menu.rs`/`pause.rs`) inserts its own `Menu` in-match, so it can't be reused
  here. Instead the main menu switches its screen to the first tab and keeps `(screen, sel)` in `Menu::over`.
  `options::step_tabs`' Back restores it (`settings_back`); from the main menu's own Settings row `over` is
  `None` and Back goes to Main as before.
- Lobby underneath: `online::poll` gates its lobby logic on the screen being Chars/Lobby/Confirm. `over::under`
  puts the lobby's screen and row back for the frame's network work, and `over::back` restores the pages after it.
  So events, picks, kicks and the host's countdown carry on as on the select. If the lobby moves (select → lobby →
  confirm) the pages stay up over the new spot. If it ends (kick, closed lobby → Online) or the match starts
  (`net.started`) the pages close with it. A first try with separate before/after systems leaked the swapped screen
  to other systems (the legend showed the roster's keys on the pages), so the swap now lives inside `poll`.
- The countdown: while the host's countdown runs, the pages' footnote ("Changes apply right away") is replaced by
  "The match is starting (n)" in blue (`over::countdown`, drawn in `options::chrome`). That footer is already laid
  out for every aspect ratio. A first version put a banner at the top centre, but it covered the tab pills in 16:9
  and an options row in portrait.
- Act `Settings` in hst-play's legend (keys "O", glyph R3). `HST_KEYS` learned `KeyO` for the shots.

## Checks
- Tests: `over::` (the select opens/backs, not while typing, the lobby running underneath incl. a move, a kick, the
  match start and the countdown text), the legend test (Settings key/glyph), `options`, `pause`, `-p hst-play legend`.
- Shots in `context/B558/run2`–`run6`: roster → O → Settings → Esc back to the roster with the moved pick kept;
  lobby → O → Settings (the pages' own legend) → Esc back to the lobby; portrait 600×900 layout; the countdown in
  the footer at 16:9 and portrait (forced with a temporary `count: 5` in `online::sample`, reverted).

## Not verified / not 1:1
- Remaster-only feature: the original has no Settings from the roster or a lobby, so there's nothing to compare.
- Not run against a real two-peer online lobby. Checked with the sample lobby (`HST_MENU=lobby`) and unit tests
  that drive `under`/`back` through a move, a kick and the match start.
- No touch/click target: the legend items aren't tappable, so on touch-only devices the pages can't be opened
  from the roster or lobby yet (the key/pad button only).
- A mod-download prompt that arrives while the pages are up only shows after Back. Its wait timer keeps running
  meanwhile.
- The roster's 3D previews are hidden while the pages are up, as on the main menu's Settings.
- "The countdown still shows/blocks ready" was read as: ready states and the countdown work unchanged underneath,
  and the countdown is shown on the pages. Readying itself happens on the lobby after Back.
- Also: `audio::tests::each_volume_silences_its_group` expected the old full-volume defaults after 8926a2b3 set music/ambience to 5; its expectations were updated so the suite passes.
