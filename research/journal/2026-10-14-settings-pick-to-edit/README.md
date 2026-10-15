# B326 — Settings rows picked before they change

The Settings pages (Display, Graphics, Sound, Online & Mods, Mobile) now work like the confirm screen's Sets and
Games pills (B212):

- ◀ ▶ on a highlighted row do nothing; ↑↓ move, L1 / R1 change tab, ○ back to the main menu as before.
- ✕ / Enter / a tap picks the row (`Menu::editing` holds the settings it opened on). Picked, its value box turns
  blue between `<` `>` (a slider's arrow gone at its end), ◀ ▶ (or a tap on the value's halves) change it,
  applied and saved at once as before ("Changes apply right away"); ↑↓ and L1 / R1 do nothing.
- ✕ (or a tap on the row) keeps it; ○ (or a tap anywhere else, `Draw::popup`) puts the whole settings back as they
  were when picked (and the chat-hidden flag with them), saved again.
- Legend (`legend.rs` acts): unpicked `↕ Option · ✕ Change · L1/R1 Page · ○ Back`; picked
  `◀▶ Change · ✕ Keep · ○ Cancel`. The legend test now also starts from a picked row on every row of every page.
- `HST_MENU=sound-edit` opens the Sound page with Music picked (shots in `context/b326/`).

The online lobby's settings row (Sets / Games / Mods) is unchanged: the task names the confirm screen's pills as the
model, and the lobby's row is navigated with ↑↓ along it and ←→ changing it (its own layout, tests in `online.rs`).
`online.rs` needed no change (Relay on the Online screen already ignores ←→).

Tests: `options::every_page` (←→ unpicked change nothing; pick, change, ↓/R1 stay, ○ restores, ✕ keeps),
`sound_sliders`, `pointer::hover_then_click_and_arrows` (tap picks, halves step, row keeps, tap off restores),
`pause::back_returns_to_pause`, `pause::open_change_back_in_a_match`, `legend::legend_lists_what_each_screen_handles`.

## Not verified / not 1:1
- None against the original: the Settings menu is ours (front-end, not 1:1 by PLAN.md); checked by `--shot` only.
- Online lobby settings row left as ←→-changes (see above); if the user wanted it picked too, that's a follow-up.
