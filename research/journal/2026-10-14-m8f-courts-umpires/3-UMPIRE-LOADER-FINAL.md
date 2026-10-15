# 2 — umpire mod loader (M8f3)

## What landed
- `crates/hst/src/mods_umpire.rs` (new): `Umpire` (id, name, source, `donor` 0..4, voice folder), `read`
  (standard 1, `kind: umpire`, donor range, voice path inside the folder), `list` (same walk as the character
  roster: `mods::dirs`, split out of `mods::list`), `find` by id, `bank` (donor disc bank + the mod's calls),
  `MatchUmpire` resource.
- `audio.rs`: `SoundBank::takes`: exact (program, key) → wav. This has to be exact because the character mods'
  `wav` map is positional and wraps (`key % len`), which would play the wrong call for a sparse umpire mod.
  `play_centre` plays a take in place of the disc's; `centre_frames` (the match's timing) leaves takes out, so a
  word lasts as long as the donor's take whatever the clip's length. `with_takes`, `play_take` (split out of
  `play_as`), test helper `takes_sounding`.
- `play/umpire_mod.rs` (new, PostStartup after `setup`): replaces slot 5's bank with `mods_umpire::bank`. The only
  play.rs changes are the `mod` line and the `add_plugins` line.
- `main.rs`: `--umpire-mod DIR`, `--umpire-mod-id ID` (looked up under `mods/` beside the ISO; not installed →
  the donor). The mod's donor goes in the chair only when no `--umpire` is given, because the menu's and online
  flags always pass the donor and that keeps the timing.
- Menu (`play/main_menu/umpire_mods.rs` + small edits): the strip runs Random, the five, then the mods (values
  `UMPIRES + 1 + i`). A mod pick shows its name with "<game>, calls as <donor>" and the donor's still. Its strip
  card is a plain lilac card with the name on it, and the confirm card shows the donor's face with the mod's name.
  △ previews the mod's calls. `venue()` gives the donor, `args()` adds `--umpire-mod DIR`. A mod pick isn't saved
  to settings.txt (Settings keeps a u8), but it carries over to the menu a match returns to. A pick past the
  installed mods → Random. Random draws only from the disc five.
- Online: `Room::umpire_mod` (id, name), `Cmd::UmpireMod` sent next to every `Venue`; `netplay::flags` adds
  `--umpire-mod-id` (not in a vanilla lobby); the lobby's umpire pill shows the mod's name.
  The handshake checks only `--umpire` (the donor), so peers with and without the mod stay compatible.

## Checks
- `an_umpire_mod_is_heard_and_never_moves_the_match` (play/umpire_mod.rs, 8 s): a bot match with the disc umpire
  and the same match with a mod (3 s clips on the first words and on program 2, 600-sample clips on the rest,
  0_13 too) give the same `state_hash` every tick through three points, and the mod's clips are heard. Mutation
  check: with `centre_frames` timing the takes it fails at tick 885.
- `calls_over_the_donor_timed_by_the_donor`, `reads_and_lists_umpire_mods`, `umpire_mods_follow_the_disc_five`,
  `flags_carry_the_umpire_mod_id`, `umpire_mod_flags`.
- Full `tools/check.sh --no-fail-fast`: 666 passed.
- Screens `HST_MENU=umpires-mod` / `confirm-umpire-mod` with a scratch mod (`context/m8f3/*.png`) at 16:9;
  live `--play --umpire-mod` logged "1 calls of its own over umpire 4's".

## Findings
- Umpire 2 (Lily)'s bank answers program 0 key 13 ("deuce again") with a 76-frame play. The standard says no
  disc umpire has that take. Either the bank wraps keys or she really has it; M8f2 should listen and correct
  §3a. The timing is the donor's either way.
- `mod_match::umpires_are_on_the_roster_and_play_a_rally` failed here because the shared `mods/` holds only
  `hst_umpire_04_anna` of the five umpire-as-player mods. It now skips unless all five are installed (its doc
  already said "skipped without the umpire mods").

## Not verified / not 1:1
- No real umpire mod to play yet (M8f2 makes the Get a Grip ones), so the clips were only test tones and
  nobody has listened to a real umpire calling a match.
- Select Umpire's picture for a mod is the donor's disc still: `previews/umpire.png` (standard §3) isn't loaded
  yet (needs an image path through the menu's sheet textures; M8f4 brings the model anyway). The menus are ours
  to design (PLAN), so there's no original to match.
- A mod pick isn't remembered across runs (settings.txt `umpire` is a disc index); within a run it is.
- Random never picks a mod.
- Online, a peer whose copy of the mod names a different donor still uses the room's umpire for timing and
  model, with its clips on top (logged).
- In-match presentation only, so no PCSX2 comparison: the original has no umpire mods.
