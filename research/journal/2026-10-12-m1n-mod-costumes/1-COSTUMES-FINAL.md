# M1n — mods listing costumes they don't have

## Where the count comes from
The select's costume count for a mod is `mod.json`'s `costumes` list (`Menu::costumes` → `Mod::costumes.len()`),
not the donor's count or the disc's 10. The mods repo's packager (`tools/rerig/package.py`) builds that list from
`glob(model/*.glb)` of the files it actually hard-links, so it never lists a costume without a model. No change
was needed there.

## Why Sam's costumes 5–8 drew nothing
That was B99's bug (a motions file keying a joint only some costumes have made costumes 4+ of many Fore! mods fail
to load), which B99 already fixed. Re-checked today against the current `../open-tennis-mods/out/mods`:
- `mods::tests::loads_every_mod` (ignored test, `HST_MODS=…`) now loads **every** costume of every mod (it used to
  load only costume 0) and fails a costume with no vertices to draw: 361 of 361 costumes of 92 mods load.
- Every baked select PNG in the main checkout's `cache/previews/SCUS_976.10-r3/m-*.png` (717 files) has a figure
  in it (alpha coverage and colour count checked). Sam's c00–c07 side by side: 4 gi outfits + 4 fundoshi
  outfits, all drawn.

## What changed (so it can't come back)
- `mods::read`: a listed costume whose `.glb` is missing is now left out with a warning, rather than the whole mod
  being rejected. The mod is still rejected if none are left.
- The select (`Menu::broken`): a mod costume whose model fails to load in the preview loader (`previews::receive`
  → `broke`) is marked. L1/R1 skip it, it shows as closed and can't be readied on, and a player still picking on it
  moves to the next costume that hasn't failed.
- Tests: `read` drops a missing costume (`mods::tests` mod.json check), and `custom_page_picks_a_mod` checks
  R1 passes a broken costume and can't ready on it.

## Not verified / not 1:1
- No in-app `--shot` cycling of all 361 costumes. The all-costume load test and the baked-PNG scan cover it, and
  the PNGs are what the cards show while scrolling.
- A player who has **already readied** on a costume that fails to load later is not unreadied (the failure only
  marks it and moves players still picking). This can only happen with a broken pack, and no current mod has one.
- The front-end doesn't have to match the original (PLAN), so there is nothing to compare there.
