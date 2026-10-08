# M1c: mods in a match (2026-10-08)

Remaster-only feature; the yardstick is the donor disc character. Iteration: this file is FINAL.

## What it does
- `--play --mod DIR [--mod-slot N]` (main.rs): `mods::read` at start (bad mod → panic with `<path>: <reason>`),
  `mods::MatchMod { slot, m }` resource; costume = `--outfits` value for that slot, modulo the mod's costumes.
- `play.rs` setup: the modded slot runs the disc path with `c = donor` (motions come from the mod's
  `CharacterData`; trajectory/serve/smash tables, shot records, timing variants, serve-miss table, `g.chars`
  = donor), then `play/mod_match.rs::apply` replaces: hand, movement stats, aim stats, reach (contact search),
  smash heights, counter power, timing stats/bias (`Timing::restat`) from `mods::tparam`; AIParam row and
  second-toss pick from `ai_row`; the voice bank from `mods::voice` (donor's bank kept when no wavs).
- `bodyhit.rs`: collision size from the mod's row (`mod_match::row`).
- Row-taking helpers split out of play.rs (`stats_of`, `aim_of`, `reach_of`); the disc fns call them.

## Verified
- `play::mod_match::tests::bot_match_with_a_mod_plays_a_rally`: headless bot singles (Carol vs
  `context/mods/test_pc00` in slot 1, rerigged pc00, left hand, SPE/reach overrides): the slot gets the mod's
  hand, stats and reach (≠ disc pc00's), the mod's skeleton; a 3-shot rally with the mod returning by frame 304.
- `--shot` (HST_AUTOPLAY, singles, mod in slot 1): `context/m1c/test_pc00_s4.png` (stage 4, left-handed pc00
  mid-rally), `fore_pc00_phoebe.png` (sweet hit), `getagrip_pc12_suzuki.png` (wav voices loaded, texture face).
- `tools/check.sh -p hst`: 56 pass.
- Test data: `context/mods/test_pc00` rebuilt as in the M1a journal (rerig.py of HST-MODS `pc00_c00.glb`);
  symlinks `fore_pc00_phoebe`, `getagrip_pc00_emi`, `getagrip_pc12_suzuki` into `~/repos/HST-MODS/out/mods`.

## Not verified / not 1:1
- Nothing to compare in the original (mods are remaster-only); only checked against the donor path.
- AI costume level: AIParam's row uses the slot's `--outfits` number as a disc outfit level; a mod costume has
  no level of its own (B40a should decide).
- Serve heights (`serve_data`) still come from character 0's row for every player (existing port gap, not mod
  specific), so a mod's serve-height overrides do nothing.
- The a/b voice-bank draw's line-up (`rng::voice_bank`) uses `--chars`, not the donor, for a modded slot.
- Panel/stats portrait and the result screen use the donor's character number (no mod portrait until B40a).
- Mod wav voices not heard against anything (M1e covers SPU-likeness); only checked that the bank loads.
- Only one mod per match (one `--mod`); B40a's select replaces the flag.
