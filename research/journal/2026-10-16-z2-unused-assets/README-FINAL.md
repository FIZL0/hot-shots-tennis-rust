# Z2 — unused loads and replacement textures

## Asset-use log (new)

`HST_ASSET_LOG=FILE hst …` appends one `kind<TAB>name` line per asset the run touches, once each:

- `disc` — a disc file read (`Iso::read`).
- `xb` — an archive entry decoded (`Archive::read`), named `DISC/PATH:entry` (nested archives by their parent entry).
- `tex-key KEY` / `tex-draw KEY` — a texture made by `textures::add` / seen on screen. "Seen" means a visible mesh's
  GsMaterial, Standard, Fx or OnTop material, or a visible UI `ImageNode`, checked every frame in `Last`. KEY is the
  `--dump-textures` key.
- `img-draw ARCHIVE:entry` — sheets, pop-ups, balloons, flare, loading art and labels made outside `textures::add`,
  tied to their entry by the decoded bytes' hash (`asset_use::source`).
- `tex-pack FILE` — a texture-pack file that replaced a disc texture.
- `snd BANK program:key` — a play through `Sound::play_as` (disc and mod sounds).
- `file PATH` — a mod file read (`modfs::read`).

Code: `hst-data/src/uselog.rs` (log, disc/entry hooks), `hst-base/src/asset_use.rs` (draw tracking), and
`hst-data/src/bin/texkeys.rs`, which prints every disc texture's key and dump name (5831 textures, 2905 unique keys).
`research/assets_used.py` folds the logs into lists of what was loaded but never used, and checks a pack's files
against the keys. `tools/asset_sweep.sh OUT N` runs the sweep. With the log off, every hook is one relaxed atomic
load.

## The sweep (context/z2/sweep1, 174 runs, old binary)

- **Matches:** all-computer, 1 game, 1 set, 300 s. Every court 1..11, singles and doubles; umpires 0..4, weathers
  0..3, characters and costumes rotating.
- **Costumes:** every disc character in all 10 costumes, 25 s each.
- **Mods:** every installed mod with a `mod.json` (83 characters, 16 courts, 8 umpires, plus the four loose ones),
  30 s each. Each run's log shows the mod's own files being read.
- **Menus:** six `HST_MASH` seeds, 150 s each.

Totals: 404 disc files, 6556 entries decoded, 2258 texture keys made, 2220 drawn, 140 sound banks played. Output is
in `context/z2/sweep1.txt`.

## Stripped (proven by the logs)

1. **Court models** (`hst/src/main.rs`, `court_models`):
   - **Before:** the load built GPU meshes, materials and textures for every model in `CMN.XB`, `GRD01.XB` and
     `HOL01.XB`: every hole variant, every season, every sky, and in doubles the clouds, which the game never moves
     or draws there.
   - **Now:** only the stems the loops below the build spawn: this hole's in-season ground and backdrops up to the
     first sky with a colour, the props the plant records resolve to, and the clouds in singles. The same set feeds
     the CPU shade parse (`raw_models`, `model_shapes`, `hole_box`), which only reads those stems.
   - **A/B** (old and new binary, every court, singles and doubles, fixed seed and weather, shot at 4 s;
     `context/z2/ab/`):
     - The same `tex-draw` set in all 22 runs.
     - 6 fewer textures made in every doubles run.
     - Pixel differences of 3–445 px out of 2.4 M, only on the moving player, ball and racket trail. Old vs old on
       court 2 singles also differs by 112 px, so this is animation timing noise.
2. **Disc faces on the score panel** (`hst-play/src/play/panel.rs`): the panel made the disc
   `face/face_CC_OO.tm2` for every player, then swapped in the face cut from the select preview where there is one.
   The sweep shows `face_02/03/04/09/10_00` made and never drawn. The disc face is now only made where there's no
   own face.

Tests: `tools/check.sh -p hst-play panel` (4 passed), `-p hst court` (11 passed), `-p hst-data` (23 passed).
`done.sh` runs the full suite and the replay tests.

## Left alone, and why

Every remaining "never used" item is something the sweep can't reach, not something nothing uses:

- **EFFCT** `cf00-kusa/metal03/netpole/suna/taki/tamari`, `impactef_drop/flat`: surface hits (net pole, water,
  sand) and drop/flat shot impacts. These are rare match events.
- **INPANE** `inpane_set`, `result_set`: only shown when a set ends with more to play; the sweep played 1-set
  matches. `i_dialog_00/01` and `inpane_pause01` are dialog and pause art. `hsm_yubi` is Hot Shot mode.
- **Menu sheets** (MENU03/05/10A/12A/12B/60/70): the front-end loads `SHEETS` at start. Pause → Settings uses
  `SHEETS[..10]`, and some screens index sheets by computed numbers. The mash reached few screens.
- **`holeend/*_h01` models** and `common/debug/arrow`: still decoded by the court's collision world (sim data).
  The court build no longer makes them.
- **Sound banks** `BGMM_04/05/06/10` (menu music for screens the mash didn't reach), `dvv_vc00d` and `galsg02b`:
  loaded by the setup RNG's pick, but no voice or crowd line came up in a short match.
- **HD pack** (`mods/hst-hd`, the user's data, outside git): none of the 4584 files is an orphan, since every key is
  a disc texture.
  - 247 files were never looked up (list in `context/z2/pack_unused.txt`). Most are menu textures: our rebuilt
    menus don't consult packs, but `--original` sends the disc sheets through `add_tim2`, which does. The rest are
    Hot Shot mode, prize and pop-up art.
  - 8 were looked up and never drawn: disc faces drawn when no preview face exists, `cf00-kusa`, `hsm_yubi`.
  - **No pack file was deleted.** "Nothing uses it" isn't proven for any of them, and deleting is irreversible.

**Z21** carries this on: drive those paths with scripted input (`--original`, Settings, Hot Shot mode, tournaments,
3-set matches, prize), re-fold the logs, and strip or delete what stays unused.

## Not verified / not 1:1

- **Coverage:** the sweep doesn't reach every mode. Tournaments, Hot Shot mode, `--original` menus, multi-set
  matches and most menu screens weren't driven; random mashing reaches few screens. This is why only the two strips
  above were made; everything else moved to Z21.
- **Draw detection:** a texture counts as drawn when a visible entity uses it in some frame (`ViewVisibility` /
  `InheritedVisibility`). A material made and drawn under another entity's handle, or an image used only through a
  custom material outside the four tracked types, would read as never drawn. The A/B compares the same tracker on
  both sides, so it isn't affected.
- **Models** are judged by their MTL's textures being drawn, not by the meshes themselves.
- **Not logged:** motions (ANI/MOR) and other non-texture data. They're logged as decoded (`xb`) but not as used.
- **Court-strip savings:** measured as textures made (−6 in doubles) and by construction (variants not built), not
  timed. Z1's load timings weren't re-run.
- **Mod costumes:** mod characters ran in their first costume only.
