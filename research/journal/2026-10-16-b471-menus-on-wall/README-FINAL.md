# B471: Mods, Settings, roster and lobby in the main menu's look

Spec: `plan/B471.md` (the concept artifact it was drawn from could not be opened; the plan's text is the spec).

## What changed

- `look.rs`: the shared kit is now the main menu's language instead of B448's navy wells. It has:
  - `top`: the coloured title band plus the hanging title tab, which is Back for the pointer.
  - `bar_y`: where the top bar's pills sit.
  - `tabs` in two styles: `Bar` (grey pills, chosen yellow) and `Strip` (on a header strip, chosen a white pill, others translucent dark), with an optional white cursor halo.
  - `panel`: off-white `#fbf9fd`, 3 px `#1d1a24` outline, 18 px corners, a coloured header strip with a dark rule.
  - Pills: `button`, `word_pill`, `outlined_pill`, `steps` (the white "‹ value ›" pill).
  - Smaller pieces: `tile`, `badge`, `heading`, `scroll_bar`.
  - Colours: PAPER/INK/GREY/YELLOW/MUTED/RULE/TILE/BLUE/MINT. The kind colours are accents only.
  - `LEGEND` is now a 6-unit gap, because the view already ends above the legend bar (`Grid::fit_at`).
- Mods (`mods_screen/grid.rs`, `mods_kind.rs`, `mods_screen.rs`, `pointer.rs`, `legend.rs`):
  - **Top bar:** red band with the "Mods" tab, then Show tabs (Everything · Installed · Updates · My uploads · Saved). L1/R1 move between them (`show_step`, stopping at the ends) and a tap opens one (`Act::Show`). The white search pill sits at the right.
  - **Grid panel strip** (kind colour):
    - Kind tabs with counts (L2/R2).
    - "‹ Newest ›" and "‹ Game: All games ›" step pills. A tap on a half steps that way; ✕ steps on.
    - Upload page: Sign in / Upload all.
    - Two rows on a narrow panel.
  - **Grid panel body:** per-game headings, then light tiles (selected white with the kind ring) carrying outlined badges (Update yellow, New light blue, Installed light green, upload states the same way).
  - **Grid panel footer:** online note left, download status right with its spinner (moved from the top bar), page words, and Upload at the right.
  - **Right panel:**
    - The kind-colour picture strip, with the name outlined bottom-left and the state badge top-left.
    - Then the costumes/hand line, the pink stats card in a dark ring, and the facts.
    - At the bottom, a yellow primary pill over two grey ones.
    - Questions stay in the column (B448) and are restyled. The page behind dims.
  - **Cursor and legend:** the cursor moves over top bar → strip → tiles → footer. The legend gained "Show" (L1/R1).
- Settings (`options.rs`):
  - **Top bar:** blue band with the "Settings" tab, and the page tabs (L1/R1) in the top bar. On a portrait window they get a row of their own.
  - **List panel:**
    - Blue strip with the page name outlined and "N options".
    - Rows are full-width grey pills (focused yellow) with a white "‹ value ›" pill.
    - Volume bars are a white track in a dark outline, filled blue.
    - Editing turns the value pill blue with white words.
    - Footnote under a rule.
  - **Side panel:** light-green strip with the option name outlined, the explanation, a CHOICES rule, and choice pills (current blue with white text, others light).
  - **Controls:**
    - Key and pad bindings are white outlined pills (blue while waiting for a key).
    - The KEYBOARD/CONTROLLER headings sit on the strip, and the count follows the page name.
    - Where the 15 rows wouldn't keep their full pitch (4:1 windows), the strip slims from 46 to 30 px.
- Roster (`roster.rs`, layout untouched):
  - Seat palette moved to light: off-white card in a 3 px dark outline (dashed for a computer player).
  - The seat colour is the header strip, inside the outline with a dark rule under it. A computer player gets the seat's light tint with dark words.
  - Dark words throughout.
  - Grey outlined costume/hand pills with white key caps.
  - Dark play-style chip, light section rail and headings.
- Lobby and confirm (`frame.rs`): `ground` is the wall, and the title is outlined.

## Checked

- Shots of the same set at 1280×720, 960×720, 1680×720 and 400×800, in `context/b471/r2/`, with the fixes re-shot in `r3/`:
  - Mods: browse, installed, a question (Remove), my uploads (Take down question), upload (Upload all), sign in.
  - Settings: display, graphics, sound, sound editing, controls, online & mods.
  - Roster: 1, 2 and 4 players.
  - Lobby and confirm.
- Everything lays out uniformly at every size, with nothing stretched. The panels stop just above the legend bar, and the tab sits clear of the top bar's first key cap.
- `tools/check.sh -p hst`: the main-menu tests pass. Tests updated for the new behaviour:
  - The sort/filter step only on a tap/✕.
  - Tiles ↑ lead to the strip's sort/filter, and the search box sits at the top bar's right.
  - The legend has "Show".
- On a loaded machine (other slots building), `upload_all` (packing threads) and `netplay::a_bot_takes_a_silent_peers_seat` each failed once in a full run. Both pass alone and neither touches this code.

## Not verified / not 1:1

- **The concept artifact:** couldn't be opened. Colours, sizes and placement come from `plan/B471.md`'s words, not from the mockup's pixels.
- **Kind tab order:** stays Characters · Courts · Umpires · Texture packs (`mods_kind::KINDS`), not the spec's Characters · Textures · Umpires · Courts. Reordering would change L2/R2 order and the tests for B373/B462, which is out of scope here.
- **Show tabs:** stop at the ends (like the kind tabs since B462) rather than wrapping.
- **Lobby and confirm:** only the ground (the wall) and the outlined title changed. Their team panels, court strip and bottom bar keep their navy/court look; the spec only asked for the wall there.
- **Store mods:** a store mod without grades shows no stats card (as before). The right panel leaves that space empty.
- **Controls strip:** the 30 px slim strip on very wide windows is ours; the spec has one strip height.
- **Menus generally:** they're ours to design (PLAN.md), so there is no comparison against the original game.
