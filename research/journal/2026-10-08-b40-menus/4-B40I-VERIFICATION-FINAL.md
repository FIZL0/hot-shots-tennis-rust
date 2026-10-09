# B40i Verification (2026-10-08)

Task: Replace B40's guesses with original game values from MENU.BIN draw code/tables or PCSX2 captures.

## Verification status

### 1. Sprite source rects and screen positions
- **Status**: NOT VERIFIED / not 1:1
- **Reason**: Layout, sizes and sprite rects measured by eye, not from MENU.BIN draw code
- **Current**: Hardcoded src rects in play/main_menu.rs
- **Needed**: Read MENU.BIN for CharacterSelect2_00..03, Confirmation_03, Option_00, titles, walls

### 2. Font glyph advance and spacing
- **Status**: PARTIALLY VERIFIED
- **Reason**: word.tm2 spacing guessed (ink width + 2 / space 10). B40e3 verified bar font is ascii.bmp, not word.tm2
- **Current**: Draw::width uses g.2 + 2.0, fallback 10.0
- **Not verified**: word.tm2 actual ink widths

### 3. SYS_SE00 keys for move/confirm/back
- **Status**: NOT VERIFIED / assumed
- **Reason**: Back sound key 1 assumed (move 0, confirm 2 checked by ear)
- **Needed**: PCSX2 capture of SYS_SE00 keys

### 4. Portrait sheet and cell for costume 9
- **Status**: NOT VERIFIED / assumed
- **Reason**: CS2_Shots_Cos_a/b assumed for costume 9
- **Current**: portrait() maps costume 9 to sheet 2 + c/8
- **Needed**: Verify against MENU.BIN

### 5. Confirm screen court order → stage number
- **Status**: NOT VERIFIED / assumed
- **Reason**: court k → stage k+1 assumed
- **Current**: let court = self.court + 1
- **Needed**: Verify court ordering

### 6. Play-style label and grade letter mapping
- **Status**: PARTIALLY VERIFIED
- **Play-style**: TParam col 7 mapping (Shift-JIS) from game data
- **Grade**: Uses NAMES sheet with grades from exe::Menu::grades()
- **Not verified**: ABCDEF[5-v] assumption

### 7. Umpire face rects and order 0-4
- **Status**: NOT VERIFIED / assumed
- **Reason**: 88.0 pixel width per umpire assumed
- **Current**: [10.0 + 88.0 * m.umpire, 394.0, 76.0, 60.0]
- **Needed**: Verify from MENU.BIN

### 8. Sets choices 1/3/5 → sets-to-win
- **Status**: VERIFIED
- **Mapping**: (n + 1) / 2 → 1, 2, 3 for sets 1, 3, 5
- **Source**: Standard tennis, game data at 0x423048

### 9. Pad numbering across processes
- **Status**: NOT VERIFIED
- **Reason**: Not tested with two real pads
- **Hand motion**: B40e1 pending

## Not verified / not 1:1 summary

All items except sets-to-win are not verified or assumed. Task too large for one unattended run. Per AGENT.md, marking as stuck.

## Sources
- PLAN.md B40i
- research/journal/2026-10-08-b40-menus/README.md
- crates/hst/src/play/main_menu.rs
