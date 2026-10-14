# B349 — character select: hand-swap hint says "Select"

The legend's hand-swap pill (`play/main_menu/legend.rs`, `glyphs`, `Act::Hand`) showed Xbox's name for the
button, "View", on Xbox and Steam Deck styles. It now says "Select" there; PlayStation style keeps "Create"
(the label printed on a PS5 pad, as Start keeps "Options"). Keyboard keycaps are unchanged. Test
`hand_hint_says_select` pins the three styles.

## Not verified / not 1:1
- No screenshot taken: a one-word label change in our own front-end (menus don't have to match the original).
