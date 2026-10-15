# B383 — Mods page: one tab row (built)

The board was approved (user 2026-10-15), so this iteration builds it.

## What changed
- **Header**: "Mods", then the one tab row: the kinds (Characters · Courts · Umpires · Texture packs, each with its
  count) between L2/R2 glyphs drawn in the legend's own art for the last device used (PS L2/R2, Switch ZL/ZR,
  Xbox LT/RT, keyboard 1/3; `Tex::Glyph`, baked by `legend.rs`). A kind with no mods (store + installed) is hidden
  and L2/R2 skip it; with only one kind the glyphs and the legend's Kind go. Then the search box, the **Upload**
  button and the downloads pill. The old page tab row and its Q/E (1/3) bubbles are gone, and so is the Tab legend entry.
- **Show** group in the rail: Everything · Installed N · Updates · My uploads · Wish list. Installed and My uploads
  are grids like Browse (Installed's heading: sort words + Add mod · Reload · Remove all; My uploads': + Refresh;
  a tile's ✕ removes / takes down in the side panel). Upload stays a list page; ○ on it goes back to the grid.
- **Search box**: never takes focus on its own. sel 0 = Search on open, after every kind switch and after every
  page switch (Show → Installed / My uploads / back). Keyboard ✕ types in place (✕/Enter/Esc or leaving the box
  stops); a pad's ✕ or a phone opens the on-screen keyboard. The page's last device is tracked in `View.pad`.
- **Navigation**: Search → (right) Upload; Upload ↓ → the heading's last word; words ↑ → Search; the rest as B372.
- **Detail panel** buttons show the real ✕ / △ / □ art (keyboard: Enter / L / Tab caps) instead of "X ^ []".
- Phone (B208): Add mod and Upload are greyed, and pressing them (or Show → My uploads) puts up the popup.

Shots: `context/B383/run1/` (all pages, keys), `context/B383/run2/` (PS / Xbox glyphs, 4:3, Texture packs, phone portrait).

## Not verified / not 1:1
- The concept board itself (claude.ai artifact 6kpFeK6ya6YUJ66BN9Bjzk): the page is a JS shell that can't be
  fetched from here, so it was built from the task line's written spec, not compared side by side with the board.
- A filter Show (Everything / Updates / Wish list) keeps the cursor on its rail option rather than going to Search.
  Only the choices that change page (Installed, My uploads, back to the store) put it on Search. This reads "after
  every kind/page switch" as page switches only; a filter isn't one.
- A kind tab counts the current page's mods, but whether it's shown depends on store + installed. So "Courts 0" can
  show on Everything when courts are installed but none are in the store (seen in the made-up `browsemods` sample).
- The glyph art follows the default key bindings: a rebound △ key (Lob) still shows its default cap in the detail
  panel and kind row. The bottom legend shows the rebinding.
- Not tried by hand with a real pad or keyboard, nor in a browser: the web file input's position on Add mod
  (`ADD_AT`) is only checked by a unit test against the drawn hit.
- The phone popup shot (`run2/phone.png`) caught the popup mid-reveal; that popup's code is unchanged.
- Texture packs is never shown in the row now, because the store doesn't carry it (M5). Its empty state is still
  there, and the tests reach it.
