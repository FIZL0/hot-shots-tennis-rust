# 1 — confirm stage + Select Court sub-screen (FINAL)

## Original (PCSX2 copy 3, save slot 2 = doubles character select; ✕ confirms, ○ back in this copy)
4× ✕ from slot 2 reaches the confirm screen (shots in `context/b192/run3`, `run4`).
- A framed window shows the chosen court's 2D backdrop with the four players standing as 3D models in a row:
  team 1 left, flame + VS in the centre, team 2 right. Under each: play style, "1P"/"2P COM" tag, a rank pill
  ("Tennis Lv 5"). Court name, surface and meters along the bottom; umpire face/name top right.
- Select Court does not open a separate screen: the bottom rows swap for a tile strip ("Select Court", 5 tiles,
  ◀ ▶) and the backdrop picture changes on hover at once (150 ms after the press it already shows; no fade).
- Backdrops: `MENU/WALL/COURTnn.XB` → `confirmation_courtNN.tm2` (568×256), nn in message court order
  (0 North Academy … 10 Miyabi; 11 is an extra Miyabi variant the menu doesn't use).

## Port (`play/main_menu.rs`, `previews.rs`)
- Confirm: `stage()` draws the court picture (cover-cropped, never stretched) in a box with the players as
  `Tex::Preview` quads (inspect's 3D rig, mods included via `previews.rs`), team 1 | flame + VS | team 2,
  labels style / "NP Name" card / device or COM rank chip. Side card keeps court name, surface, meters (now
  `court_meters`), umpire.
- Select Court: `court_screen()` — the hovered court's picture fills the view under the title bar, a card with
  name + meters top left, the existing picker strip at the bottom (user asked for a sub-screen; ours).
- `previews.rs`: quads also for `Screen::Confirm`; a preview respawns when its rect size changes.
- `HST_MENU=confirm-court` (Nakagawa, umpire Lily) for shots.
- Shots checked at 16:9 and 4:3 (`context/b192/port_court_169/43.png`, `port_picker_169/43.png`).

## Not verified / not 1:1
- Player pose: inspect's menu pose, not the original confirm screen's motion; the confirm draw code in MENU.BIN
  wasn't read (menus not 1:1; time).
- Model placement, size, camera and light are ours (inspect's), matched by eye against the screenshots only.
- Court selection is a full sub-screen (task asked for it); the original swaps the rows in place over the confirm.
- No transition: the picture swaps at once on hover, as in the original; no open/close animation measured.
- Random court has no picture on disc: dark green field with a faint "?" tile (ours).
- Players aren't drawn on the court sub-screen (the original keeps them visible behind the strip).
- Umpire is still the 2D face (M10's 3D umpire not done).
