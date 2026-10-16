# 2 — every lob shows the yellow marker (user decision, FINAL)

After hearing that the original marks only lobs coming at a human, the user decided (2026-10-16): "just always
draw the lobs". This is a layered addition, not the original.

## What changed
`play/smash_marker.rs` `receivers`, called from `strike` in place of the inline receiver filter:
- **Any shot but a lob:** the original's candidates, the humans on the receiving team. Unchanged.
- **A lob:**
  - Candidates are the receiving team's humans or, when it has none, its players (bots). Two on one team give
    the midpoint, as the original does for two humans.
  - Each `[top, middle]` is capped at the lob's own predicted peak before its first bounce. A low lob (a mod
    character's lob records, a framed lob) then still places a point. A lob above the top keeps the original's
    heights, so a lob onto a human lands where the original puts it.
- The yellow marker is presentation only: the match reads just the red one (`g.marks.red`). The full suite
  passes (805 tests, replays bit-exact).

## Test
`every_lob_shows_the_marker`: 1P is a human seat pressing only △ (✕ to serve), against a bot. In 2 minutes:
- All 5 of 1P's lobs place the marker before they bounce.
- No other 1P shot sets up a search.
- Every opponent shot from the return on sets one up.

## Not verified / not 1:1
- Not 1:1 on purpose: lobs onto bot-only teams and lobs below a smash top are marked. The original marks neither.
- Not tested with Sophie (an Out of Bounds mod character). The test can't load mods. The peak cap is what makes
  her lobs place regardless of her records.
- A lob still places no point if it comes down within 1.5 m of the net or moving back toward it (the original's
  search rule, kept).
- No screenshot of the port showing the marker on one's own lob. The drawing side is unchanged (N4a, B91).
