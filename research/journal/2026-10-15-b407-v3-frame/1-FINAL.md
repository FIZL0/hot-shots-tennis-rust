# B407 iteration 1 — FINAL

## What moved into `main_menu/frame.rs`
- The board's colours (seat RING/CARD/WELL/SOFT, style gold, pink/rule/grades, navy, leaf, slate, ready green,
  paper, caption, the chosen pill's gold fill) and the frame's sizes (SIDE_W, BAR_H, VS, GAP, CARD_MAX_W, LABEL_H,
  pill height/gap).
- The geometry: `teams`, `portrait_teams_h` (the portrait cards' height both `*_geo` computed), and `side_parts::<N>`.
  `side_parts::<N>` lays out N pills two by two, with an odd last one across (the lobby's Umpire). The court-card height
  and the last part's least height are arguments (confirm 116/50, lobby 88/56).
- The parts: `dotted`, `team_labels`, `vs_disc`, `court_card`, `setting_pill`, `chip` + `dashes` (a `Chip` look
  enum, so the lobby maps its seat `State` onto it and confirm uses Ready/Com), `seat_tag`, `rows` (a card's rows
  from the foot up), `stand_in`, `card_foot` (costume/hand chips, the style, the stats card), `style_of`,
  `preview` (with the lobby's `hide`), `stats` and `tick`.
- Each screen keeps what only it has. The lobby keeps its seat states, `teams_stacked`, the invite box, ✕, YOU, the
  focus ring and Start/Ready. Confirm keeps the umpire card, the picked pill, the help words and Start the match!.
- `teams`' test moved with it. A new test covers `side_parts` with 4 and 5 pills.

## Check
- `tools/check.sh -p hst --bin hst -- main_menu`: 123 passed.
- A/B shots of the old and new binaries (`HST_MENU=confirm, confirm-random, sets, lobby-bot, lobby-wait, lobby-guest`)
  at 1280×720 and 720×1280 are in `context/b407/run1/`. The only pixels that differ are the 3D preview characters
  (their idle motion is at a different frame in each shot) and the confirm screen's flame. The frame, cards, chips,
  pills, stats and bars are identical.

## Not verified / not 1:1
- None. This is a refactor of our own front-end (no original to match), and the screens are pixel-identical apart
  from the animated parts.
