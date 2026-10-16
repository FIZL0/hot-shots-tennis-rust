# B430 — pause menu names an umpire mod by its own name

**Bug:** with a mod umpire in the chair, the pause screen's venue caption (B293) still read the donor's disc name
("Umpire Anna"), because `play/menu.rs` `setup` built it from `--umpire` only.

**Fix:** like B401's court mod, `setup` takes `Option<Res<MatchUmpire>>` and, when one is there, captions it from the
mod's `name` via the new `pause::umpire_caption` ("Umpire <name>", a leading "Umpire " in the mod's name dropped so
"Umpire 00" doesn't double up). The disc path goes through the same helper, unchanged output.

**Checked:** unit test `umpire_caption_names_a_mod_by_its_own_name`; a `--shot` of the pause screen
(`HST_PAUSE=4 tools/shot.sh … --play --court 1 --umpire 4 --umpire-mod <mod named "Gloria">`) shows
"North Academy Grounds / Umpire Gloria" (shot in `context/b430/pause2.png`).

## Not verified / not 1:1
- No comparison with the original: umpire mods and this caption are ours (front-end, not a spec).
- Online rooms: the caption follows whatever `MatchUmpire` this peer has (the room's id found under `mods/`);
  not run with two peers.
