# B399 — button and label wording

Went through the main menus' legend words (`legend::acts`), the name picker, Browse mods, the online lobby's ✕ and
the pause / stats legends, and the drawn strings in `main_menu/`, for words that read wrong or differ between screens.

Changed:
- Confirm screen's court / sets / games / umpire picker: ✕ "Keep" → "Select" (the user's report).
- Settings rows being edited: ✕ "Keep" → "Select", same as the picker.
- Seat assignment: ↑↓ "Pick" → "Move", like every other list.
- Name picker list: ↑↓ "Choose" → "Move", ✕ "OK" → "Select". Typing grid: ✕ says what it does under the cursor:
  "Type" on a letter, "OK" on OK, "Back" on Back (it said "OK" on every key).
- Browse mods grid: the arrows' "Browse" → "Move", like the Mods page's other tabs.
- Online lobby, empty seat: ✕ "Add Computer" → "Add COM", as seat assignment's △ says.

Left as they are (read fine): Confirm/Main ✕ "Choose", ○ "Back"/"Cancel"/"Leave" (they do different things),
lobby's "Not Ready" button vs a seat's "Not ready" status, the update row's "Keeps: Settings, mods, ISO" fact,
the pause menu's "Resume the Match" / "Return to the Menu" (the original's words).

Tests: `tools/check.sh -p hst main_menu` (112 passed; Browse's legend test updated).

## Not verified / not 1:1
- Not compared against the original: these are our front-end's words (PLAN: menus are ours to design), and nothing
  here touches gameplay. No screenshots taken: only legend text changed.
