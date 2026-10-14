# B271 — touch L1/R1 emote

- `play/emote.rs touch`: the touch pad (gilrs names a web pad by its `id`, web/touch.js sets
  `Touch controls (STANDARD GAMEPAD)`) held L1 = left flick (emote 2, `gu`), R1 = right flick (3, `di`).
- `play.rs read_input`: the query also reads the pad's `Name`; `s.emote` = flick, else `touch`. It goes into the
  same `SlotPad::emote` as the right stick, so the wire, replays and `emote::tick`'s rules are unchanged.
- Menus read L1/R1 themselves (main_menu), not through `read_input`: unchanged. A real pad's name never matches,
  so its shoulders don't emote; L1/R1 have no match binding by default.
- Test: `flicks` checks the name match; `cargo check --target wasm32-unknown-unknown` builds.

## Not verified / not 1:1
- Not tried on a phone / in browser touch emulation (unattended run, no browser driving here); relies on
  gilrs-core's wasm backend using `gamepad.id()` as the name (checked in its source).
- Emotes are ours (B145), so nothing to match against the original.
