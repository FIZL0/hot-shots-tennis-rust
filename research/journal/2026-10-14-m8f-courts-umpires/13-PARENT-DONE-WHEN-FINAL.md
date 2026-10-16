# M8f — parent's done-when checked (all sub-tasks landed)

All of M8f1–M8f10 (and the follow-ups M8f5a1–a5, M8f10a1–a4, M8f51, B401, M111) are in DONE.md, so this
iteration only checks the parent's done-when against the current build (release `hst` at 14bbd0ee).

## The NPC clause: changed by the user since the split
M112 (user 2026-10-15) dropped Get a Grip's people on every Get a Grip court: its chibi spectators look out of
place next to HST's cast. The gallery exporter is gone, `mods_court::read` ignores `npcs.json` on Get a Grip packs,
and none of the 16 packs in `out/mods` has one. The people pipeline from M8f7 (`play/npcs/court_people.rs`,
standard §2b) is still in place for other court mods, and `a_court_mod_is_seen_and_heard_and_never_moves_the_match`
tests it with stand-ins. So on a Get a Grip court, "its NPCs" now means the people of its layout court (HST's
walkers and ball kids). That is what the user asked for, not a gap.

## Shots (`context/M8f/run1/`, 1280×720, `HST_WEATHER_SEED=1234`)
- `match.png`: `--play --singles --court-mod mods/hst-mods/gag_c06_canada --umpire-mod mods/hst-mods/gag_ump00_natasha
  --shot-at 8`. Tennis Fields Club's court, props and snow, with Natasha in the chair.
- `match-c01.png`: the same on `gag_c01_yamate` with `gag_ump02_kanani`, at 4 s. Kanani is in the chair.
- `browse-courts.png` / `browse-umpires.png`: `HST_MENU=browsemods HST_KIND=1|2`. Courts 16 and Umpires 8, kept
  apart from Characters (the sample store is built from the installed mods, so Characters is 0 here).
- `court-select.png` / `umpire-select.png`: the installed ones after the disc courts and umpires.
- Pitfall: `--court-mod` / `--umpire-mod` take the pack's **folder**, not its id. With an id the read fails
  silently (the warning comes before logging starts) and the match draws an empty grey world. The default
  `--shot-at` of 0.5 s is also too early to see the court.

## Voice
- `play::umpire_mod::an_umpire_mod_is_heard_and_never_moves_the_match` passes: the mod's clips play in place of the
  donor's, and the match hashes the same tick for tick.
- `play::court_mod_match::a_court_mod_is_seen_and_heard_and_never_moves_the_match` passes.
- `gag_ump00_natasha/voice/` ships all 24 calls (M8f2).

## Not verified / not 1:1
- **The voice wasn't heard in a recorded match.** The shot can't carry sound, and no audio capture of a
  `--play` match with a Get a Grip pack was made. The test above covers the playback path with a stand-in pack;
  the real packs' calls were transcribed against HST's calls in M8f2.
- **No live upload or download of these mods against the Hugging Face store.** Publishing is outward-facing and
  this run is unattended (as in M8f9). Browse was checked on the offline sample store.
- **Get a Grip's own court people aren't shown**, by the user's decision (M112). Its ball kids, line judges and
  mascots never existed in its data anyway (M8f7).
- Everything else that isn't 1:1 (NPC cue timing, card lighting, Get a Grip's encrypted program) is listed in
  the sub-tasks' entries. Nothing here touches gameplay.
