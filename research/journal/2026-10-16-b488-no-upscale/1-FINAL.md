# B488 — `"upscale": false` mods skip upscaling

- `mods::Mod::upscale` (mod.json `upscale`, absent = true, not a bool = refused). `upscaled_dir` returns None for
  such a mod, so its costumes and racket draw their embedded textures whatever the Upscaled Textures setting.
  Court/umpire mods (`upscaled_in`) unchanged: the field is a character-standard one.
- Store: `Meta::no_upscale` (written by `package` from the mod, skipped in meta.json when false so older uploads
  and other mods are unchanged). `install` with Upscaled Downloads on still takes the ranged path (no upscaled set)
  for such a mod.
- opentennis-mods: `tools/rerig/package.py` already wrote `"upscale": false` for every Fore mod (c274c45), and all
  24 `out/mods/fore_pc*` mod.json have it (none has a `textures/upscaled/`). Standard §3/§6 updated to say the game
  now ignores the set and the store never downloads it (mods repo commit, not pushed: the run doesn't push).
- Tests: `checks_the_manifest` (false/true/absent/bad), `package_then_install` (meta carries it, round trip, old
  meta = wants upscaling).

## Not verified / not 1:1
- The live store: the Fore mods already up there were uploaded before `no_upscale` existed, so their meta.json
  lacks it until re-uploaded; with Upscaled Downloads on they download whole, which costs nothing extra because
  their zips have no upscaled set. Not checked against the network (no upload from an unattended run).
- Original game: not applicable (mods are an addition).
