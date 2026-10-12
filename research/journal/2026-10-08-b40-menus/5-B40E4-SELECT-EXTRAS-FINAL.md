# B40e4 — character select extras (FINAL)

`crates/hst/src/play/main_menu.rs`. Reference shots of the original: `context/b40e4/run1`, `run2` (copy 1, Game
Selection → Doubles → character select, ✕ confirms; ○ backs out to Game Selection). Remaster shots:
`context/b40e4/coms.png` (16:9, `HST_MENU=coms`), `chars43.png` (4:3).

## Done
- **BEG/INT/EXP group frames** around the face grid: a white rounded frame (CharacterSelect2_00 [154,274,60,60],
  9-sliced), a tall pill on the left (fill [392,400,28,76] tinted green/blue/red, outline [424,400,28,76], 3-sliced)
  and the letters BEG [470,401,13,38], INT [494,401,13,38], EXP [470,440,13,39].
- **Header key hints**: △ [228,4,24,24], L1 [160,9,32,19] and R1 [160,73,32,19] come from info.tm2 (MENU12A). "Show Attributes" [0,4,104,16]
  and "Select Costume" [0,27,132,18] come from KeyAssign_CharacterSelect2. They are pinned with Anchor::Sides.
- **Costume number on a shirt icon** (CS0 [471,66,38,28], tinted in the player's colour), as in the original.
- **Hand tag** on human cards: "Right hander" [60,160,76,22] / "Left hander" [60,184,76,22], from TParam.csv
  column 6 (right only if the cell is exactly 右, the same rule as `play.rs character_hand`). The test checks that
  Carol (6) and Will (11) are the only lefties.
- **COM "Tennis Lv N" pills**: Confirmation_04 (MENU12B) has the labels in 96×24 cells, 9 to a column, and the pill
  is [0,224,112,24] (fill) / [112,224,112,24] (outline). The label is the AIParam `rank` of the bot's row,
  `Choice::new(menu_row(char, costume), char, doubles).row`, which is the same rank the serve panel shows. The test
  `com_ranks_and_hands` asserts that JJ costumes 1/2 → Lv 4 (rank 1) and Cody costume 1 → Lv 5 (rank 0), which
  matches the original's shots in run2.
- `HST_MENU=coms` opens the select with P1 already readied so the COM cards show.

## Not verified / not 1:1
- In the original, the L1 and R1 hint boxes alternate. Here both are shown side by side because the period was
  not measured.
- **Select hand toggle** is not done. It is left to B79, because the match has no hand override yet, and a toggle
  that does nothing in the match would mislead. On the original (Ashley, run2), pressing Select changed neither the
  card's tag nor the RAM I watched, so the toggle's behaviour wasn't confirmed either.
- Mod characters get no hand tag, because `mods::tparam` decodes Shift-JIS lossily and the 右 test can't run on it.
- Positions, sizes and group tints are by eye from the shots. This is allowed, since menus are ours to design
  (PLAN 2026-10-12).
- The "Ready to go!" banner is kept instead of the original's OK! stamp. The group interiors are not darkened. The
  card view's separate "R1" hint is not distinct.
- The existing "Tab / Square: N custom characters" line sits on the INT frame's top edge.
