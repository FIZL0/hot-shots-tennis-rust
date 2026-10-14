# B265 — no "Ready to go!" on computer cards

Character select (local and the online lobby's): the "Ready to go!" label sat at y+97 over a COM card's rank pill
(y+114) and style label (y+94). It's now drawn only on human cards (`human` = a local seat or a remote human),
`play/main_menu.rs` character card draw. A ready COM card still shows its style, rank pill, name and "COM" tag;
"Not selectable" is unchanged.

## Not verified / not 1:1
- Not compared to a screenshot of the original: front-end is ours to design (PLAN.md, user 2026-10-12); the fix is a
  draw condition only. No `--shot` taken.
