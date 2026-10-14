# B202 — child-button submenus on the Mods pages

Our own design (PLAN: menus aren't 1:1), replacing `View::armed` and its "press again to …" statuses.

- `play/main_menu/child.rs`: the reusable widget. `Child::new(row, &["Remove"])` adds Cancel last; `step` moves
  (↑↓ wrap), ✕ returns `Got::Pick(i)` (Cancel closes), ○/Back closes. `draw` dims the screen above the description
  bar, redraws the row chosen, puts the buttons in a column 14 px right of the row (kept inside the 4:3 screen and
  above the bar, centre-anchored so they stay on screen at any aspect), moves the hand onto them and calls
  `Draw::popup(Back)`: the pointer hits only the buttons, a tap elsewhere is Back.
- `mods_screen.rs`: Installed mod → Remove; Upload's mods → Upload (the rule in the status); Sign in → Open token
  page / Paste token (Open keeps the child open on Paste; a paste with no token reopens it on Paste); Signed in →
  Sign out; My public mods → Take down (a PR) / Ask to delete (merged). `Menu::mods_cursor` makes the pointer's
  row hover/tap move the child's button while it's open (`pointer.rs`), replacing `mods_choose`.
- Tests: `removes_from_the_child` (open, ○, Cancel, Remove; pointer: only the buttons hit, outside = Back, hover
  moves the button, tap removes), `browse_and_upload_pages` (Upload / Sign out / Sign in children, Paste token),
  `my_public_mods`, `hand_added_mods_stay`.
- Shots (`context/b202/`, 4:3 960×720, 16:9 1280×720, 21:9 1680×720): `HST_MENU=addmods HST_CHILD=1` (Remove
  open), `uploadmods` (Upload open), `mymods` (Take down open), `signinmods` (new: Sign in's buttons open). All
  laid out right at each aspect; the child covers the Browse/Mine tags and part of the panel's name, as intended
  (drawn over the list, everything else dimmed).

## Not verified / not 1:1
- Not 1:1 by design (our own menu). Not tried in the browser build or on a real touch screen; the pointer path is
  covered by the unit tests (hit rects, outside act, hover/tap through `Menu::point`).
- The real Hugging Face sign-in/upload/take-down calls weren't run (they need an account: M17/M19's human checks
  cover them); only the menu flow up to those calls is tested.
