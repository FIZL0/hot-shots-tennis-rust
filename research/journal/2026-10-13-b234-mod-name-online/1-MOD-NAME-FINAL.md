# B234 — online screens name a mod by its own name, not its donor's

Cause: the select's live places (`net::Live`) carried only the mod's id. A peer without that mod shows its donor
(`Menu::align`), and the select card then drew the donor's sheet name (`CHAR_NAMES`). The lobby seat for a picked
mod already used the pick's `Wire::name`; while still choosing on a mod it only said "Picking...".

Fix:
- `Live::name` (serde default, left out when empty, so older peers still parse): the mod's own name from its pack.
- `Menu::remote_mod_name(p)`: another machine's player on a mod this one doesn't have → the name, from the live
  place or the seat pick's `Wire`. The select card draws it instead of the donor's sheet name.
- Lobby seat: a player still choosing on a mod shows the mod's name dimmed (as a disc character is shown).

Test: `online::tests::mod_name_not_donor` — the peer with the mod sends the name; the peer without it gets it from
the live place and from the seat pick; with the mod installed it's drawn as this machine's own.

## Not verified / not 1:1
- Not tried across two real machines (the unit test covers both peers' state). The front-end isn't 1:1 by rule.
- The confirm screen (offline only) still names via `m.pick`; not reached online, so unchanged.
