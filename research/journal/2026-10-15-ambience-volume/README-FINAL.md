# M111 — court ambience level and an Ambience slider

## Loudness
A court mod's ambience layers (Get a Grip's `stream_SE/env_coNN` loop, from cNN's sound archive) were levelled to
0.7 × the disc BGMs' RMS (0.069): ~0.048 RMS, a bed of birds, crowd and wind as loud as the music.
The stock courts' only continuous ambience is court 2's surf (court bank program 7 key 25, four loops at spawn
volumes 43/32/32/43). Through our mixer, 10 s from the bearings its sweep covers: 0.0174 (90/135/225/270),
0.0196 (45/135/225/315), 0.0215 (all ahead). The other program-7 sounds are short one-shots (0.02–0.11 RMS while
they sound, at 0x80 before distance falloff), so they're rare and add little loudness overall.
→ `Track::AMBIENCE_RMS = 0.02`: a mod's layers, all together, now sit at the stock surf's level (−7.7 dB from
before, ~10 dB under the BGM). Pinned by `audio::tests::court_ambience_loudness`.
The export script (`../opentennis-mods tools/psp2gltf/export_getagrip_courts.py`) needs no change: the engine
levels each layer by its own RMS, so the source level doesn't matter.

## Slider
New volume group `Group::Ambience` (master × ambience): court bank program 7 plays (`Group::of_play`: slot 0,
program 7 — creatures, emitters, surf, chimes) and court mod ambience loops. Settings → Sound → Ambience
(`ambience_volume` in settings.txt; old files without it read as 10). Rain stays under Sound Effects (weather),
as the SFX row says. Shots: `context/M111/run1/sound*.png` (16:9 and 9:16) — the row lays out at both.

## Not verified / not 1:1
- The level is matched to our own mixer's rendering of court 2's surf, not to a PCSX2 capture of the original's
  output: presentation (PLAN: in-match SFX feel right by eye/ear), and the mixer is the same one stock courts use.
- Not listened to in a live match (unattended run); the relation to the crowd is from the numbers above.
- `court_mod_match::a_court_mod_is_seen_and_heard_and_never_moves_the_match` fails in this run: the shared
  `mods/hst-mods/gag_c01_yamate` was re-exported at 23:04 by another session without its `npcs.json` (0 people);
  unrelated to this change (its ambience/hash checks pass before that assertion).
