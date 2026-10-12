# B74a — season light rows

## What the original does
The envir load (court manager, file read to manager +0x550) checks the version (0x14a), copies the file in, then
loops seasons 1–3 and copies **season 0's** rows over each: the time-of-day threshold row (file 0x10, stride 0x30),
both light sub-rows (0xd0, stride 0x70, 2 × 0x38), the fog row (0x290, stride 0x60), plus the 0x410/0x9a0-area rows
B74 saw. The source is always row 0, no season test. The per-frame light, fog and clear readers then index by the
manager's season (+0x7c), so they read row 0's values whatever the season: singles (season 1) looks like doubles.

## RAM
`research/envir_season_check.py` on `context/ram/{s03,s04,s05,s08,s09,npc_c01,npc_c02}.bin` (courts 4, 10, 1, 2):
every threshold, light and fog row of seasons 1–3 in RAM equals the file's row 0, none equals its own file row (all
differ in the file).

## Port
`gs.rs`: `light_rgb` (and so `court_light`, `model_light`, `player_light`, `court_intensity`, `court_clear`) reads
threshold 0x10 and light row 0xd0 + t·0x38; `court_fog` reads fog row 0x290. The season parameters are gone from these
functions (and `CourtLook::season`, now unused). Singles courts now get row 0's light/fog/clear (P17e's singles render
`c10s.png`, clear (30, 53, 108), was season 1's rows and is now wrong by the same logic; doubles unchanged).

## Not verified / not 1:1
- Every RAM dump available is season 0 (manager +0x7c = 0), so the copy is seen in RAM only for a season-0 load; that
  it is the same for season 1 rests on the decompile (fixed source row, no season branch). A singles state needs a
  menu drive (as B74b); not done.
- No singles screenshot compared against the original (same reason).
