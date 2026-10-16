# M118 — every court, umpire and character mod has up-to-date upscales; store re-uploaded

## Tool
- `tools/upscale_mods.py --check [DIR…]`: per mod, each `.glb` image (costumes, `racket.glb`; courts
  `court`/`gimmick`/`sky.glb` + `npcs.json` people; umpires `umpire.glb`) whose
  `textures/upscaled/<stem>/<name>.png` is missing or no longer matches `sources.json`; exit 1 if any. It also
  counts *unused* upscales: PNGs no current `.glb` image names (the game never draws them, but they ship in the zip).
- `--prune [DIR…]`: deletes those and their `sources.json` entries.
- `tools/upscale_mods.sh` is the user's untracked wrapper in the main checkout (model
  `~/.local/bin/chainner-app/HDcube4/Models/HDcube/4xHDcube4.pth`); I ran `upscale_mods.py` with the same model
  directly and didn't commit the wrapper (an untracked file of the same name in main would block the runner's merge).
- `hst-base` `modstore::tests::reupload` (`#[ignore]`, network, writes to the store): re-uploads the folders in
  `HST_REUPLOAD` with the game's own `package`/`upload` as the account in `HST_TOKEN`'s `hf-token.txt`, onto that
  account's open PR (same as the Mods page's Upload, minus the select's stats/card; the existing metas had neither,
  and a commit onto the PR keeps its preview/thumb).

## Counts — build folder `../opentennis-mods/out/mods` (107 mods: 41 `upscale: false`, 82 checked)
| | missing/stale | unused |
|---|---|---|
| before | 69 in 23 mods | 889 in 50 mods |
| after | 0 | 0 |

- Missing: the 3 `racket.glb` images of `getagrip_pc01`–`09` and `opentee2_pc04`–`13` (the rebuilt rackets' images are
  `racketNN_*` with a new number; the old `racket00_*` upscales stayed), and the 3 `c00` images of all five
  `hst_umpire_*` character mods (none ever upscaled). chaiNNer run: 69/69 in one batch.
- Unused (pruned, backup in `context/M118/orphans-backup.tgz`): the Get a Grip gallery people (`gall00`–`06`, dropped
  from the courts in M112), gimmick/court/stands images of rebuilt court models, and the old racket images of all 14
  published Get a Grip players + the opentee2 players.

## Store (HF `FIZL0/opentennis-mods`, all open PRs by hotshotsgaming)
Downloaded every published getagrip/gag/hst_umpire zip (40) and ran `--check` on them:
- before: 30 missing in 10 mods (getagrip_pc01–09 rackets, hst_umpire_03_robottennis), 829 unused in 30 mods
- re-uploaded the 31 changed + published mods (16 `gag_c*` courts, `getagrip_pc01`–`14`, `hst_umpire_03_robottennis`)
  onto their PRs (82…135); each store zip differed from the build only under `textures/upscaled/` (diffed).
- after (downloaded again): 0 missing, 0 unused in all 40.
- The 8 `gag_ump*` umpires were complete before and unchanged. Unpublished changed mods (the other four
  `hst_umpire_*`, `opentee2_*`) were only fixed locally.

## Not verified / not 1:1
- Didn't look at the new racket/umpire upscales in the game (upscale output of the same chain the other mods use;
  sizes checked: 4×).
- `getagrip_pc01_ban` and `getagrip_pc04_brad` also have older open PRs (76, 77) with the old upload; the upload
  goes onto the newest (108, 112) as the game's Upload does. Left alone.
- This worktree's `mods/` installs (`getagrip_pc01_ban`, `_pc04_brad`, `hst_umpire_04_anna`) are old store
  downloads with the gap; a re-install from the store fixes them. Not touched.
