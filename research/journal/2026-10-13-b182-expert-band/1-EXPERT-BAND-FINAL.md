# 1 — expert band (FINAL)

## The expert rows
The character select's bottom row (`play/main_menu.rs` GRID: "beginner row, intermediate cluster, expert row") is
TParam rows 8 Miranda (オール), 9 Kent (ベース), 10 Lola (ネット), 11 Will (オール). Their range per column (min–max),
against all 14 rows:

| column | experts | all 14 |
|---|---|---|
| Serv / Strk / Voley POW | 9–13 / 10–16 / 9–16 | 3–14 / 4–16 / 2–16 |
| LOW / V LOW POW | 10–16 / 9–16 | 4–16 / 2–16 |
| Lob POW / Lob POW2 | 10–12 / 10–14 | 5–12 / 5–14 |
| Top / Slice / Drop SPIN | 9–15 / 9–14 / 8–16 | 4–15 / 3–14 / 3–16 |
| Strk / Voley / Serv CON | 22–40 / 20–35 / 20–35 | 18–40 / 20–35 / 20–35 |
| timing IMP early / late | 1/1/8 … 3/0/7 (both) | 1/1/8 … 9/0/10 / 1/1/8 … 4/0/8 |
| SPE / Agili / STA | 10–12 / 25–30 / 40 | 9–12 / 10–40 / 35–40 |
| reach (cm), dives 2×/4× | 80–90 | 60–100 |

Experts all share 計 ≥ 808 (beginners 743–784). Agili's direction (higher = better?) isn't known: the experts sit at
25–30, beginners at 40, JJ at 10; the remap keeps a mod's order on it whichever way it reads.

## The rule (open-tennis-mods `tools/rerig/gag_stats.py` `expert`)
For each band column (the power, spin, control, timing-window, speed, stamina and reach columns above): the mod's
current value (its `params.override`, else its base row's) has a place among HST's 14 rows of that column (average
rank, ties share it); the new value sits at the same place between the experts' lowest and highest (interpolated over
the 4 sorted expert values, rounded; `a/b/c` timing cells are whole expert cells). So a mod that was the strongest
server stays the strongest, just between Miranda and Will instead of between Ashley and Big Chief. Dives are reset to
2× / 4× the new reach (as every HST row). `ai_row`: the expert of the mod's play style (`タイプ` override, else its old
AI row's type), the donor's sex, nearest the donor's Bip01 height; ビッグ (no expert has it) takes the nearest expert
of the same sex. Each mod records `params_band: "expert"` and `params_band_why` (old → new per column), so the
step runs once; `params_why` keeps the source-stat reasoning for the old value.

Wired into `package.py` (game mods) and `hst_umpires.py` (umpire mods) before they write `mod.json`;
`python3 tools/rerig/gag_stats.py [out/mods]` applied it in place to the 92 packed mods (90 changed, Anya and Wendy
skipped by id; the 5 umpire mods included — they're playable mod characters) and asserts every band value and
ai_row is inside the experts' own. Committed in open-tennis-mods 56aaf18. The remaster's `mods/hst-mods` is a symlink to
`open-tennis-mods/out/mods`, so the local game sees the new packs at once; `mods.rs` needed no change (params are read as
before).

## Checks
- `HST_MODS=…/out/mods tools/check.sh -p hst --bin hst lists_the_custom_roster`: every pack still loads (passed).
- Second run of the script changes nothing (idempotent), and its band asserts pass.
- Disc characters, TParam and the replay tests are untouched (no sim / disc code changed).

## Not verified / not 1:1
- Not played: no match was played with a rebalanced mod to feel the balance (the change is data only, within values
  the game's own experts use; no PCSX2 check is possible — mods don't exist in the original).
- Agili's meaning (higher = better or worse) is unknown; the remap keeps order either way.
- Store copies (Hugging Face) of these mods still carry the old stats until they're re-uploaded; online peers with
  an old pack and a new pack of the same mod get a gameplay-hash mismatch (`handshake.rs`) until both update.
- Ties at a column's top cap a few mods below the expert maximum (e.g. Serv CON 35 → 33): the average-rank rule.
