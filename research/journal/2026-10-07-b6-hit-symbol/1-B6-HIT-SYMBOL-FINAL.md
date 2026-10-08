# B6 — hit symbol ○/✕ swapped

The symbol is the glow disc (P9e), not the impact model: `impactef_top.tm2` is an orange ○ and
`impactef_slice.tm2` a purple ✕ (the impact models and sparks are purple for top, orange for slice).

The game's effect texture list (fx+0x50, name table in the GAME overlay) is
`ballrolling, impactef_slice, impactef_top, impactef_flat, impactef_lob, impactef_drop, toptubu00 …`, so glow
texture 0 (topspin, and flat) is `impactef_slice` = ✕ and texture 1 (slice, and drop) is `impactef_top` = ○.
P9e's test matched the texture *index*; the port then loaded the files by kind name and swapped the two.

Fix: `effects.rs` `load_flight` loads slice, top, flat, lob, drop in that order. Not checked live: the list order
comes straight from the overlay's name table, and the textures were viewed with `tm2png`.
