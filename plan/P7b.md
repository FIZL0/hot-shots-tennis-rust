# P7b

- [ ] **P7b — All 14 characters' running, own bodies (in P7).** Record each of the 14 characters as P1 with its own model (menu selection or a save per character) with a scripted vpad run, dump each one's pelvis rows, and replay through `human_pad_replay`. Also: the hand +0x12b4 comes from the model object flag (+0x135), not TParam col 6 (Carol is +1 in RAM though TParam says 左): find its source and fix play.rs `character_hand`.
