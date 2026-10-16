# B458 — site Settings & files: text wraps, never spills out of a card

**Cause.** The Storage card's usage legend was five `white-space: nowrap` spans with nothing between them, so the
whole legend was one unbreakable line and ran past the card ("Reports & se…" cut by the panel). Now the `<p>` is
`.legend`, a flex-wrap row; each swatch + label stays together (`nowrap` per item) and items wrap onto more lines.

**The rest of the page** (`tools/web_site_wrap.mjs`: headless chromium, web/ served locally, a mod with a very long
name and an unbroken source URL put in OPFS; widths 280/320/360/414/600/1024; every element of `#disc` must stay
inside its panel and the page, no sideways scroll, nothing cut off by an overflow-hidden box; legend must wrap at
320 px). Against the old CSS/JS it failed at every width; now all pass. Two more fixes it found:

- Installed mods: a mod's name was clipped with an ellipsis and its source URL ran out of the card. Both now wrap
  (`overflow-wrap: anywhere`); the row itself wraps so on a narrow card the name gets its own line, Remove at the right.
- At 280 px the Username input + Save (and the Hugging Face rows) ran ~25 px past the Player card: `.rows`' grid
  column is now `minmax(0, 1fr)` and `.r .end` may shrink (`min-width: 0; max-width: 100%`), so the input narrows.

Screenshots: `context/B458/run4/` (storage-/modlist-<width>.png). Run: `node tools/web_site_wrap.mjs [DIR]`.

## Not verified / not 1:1
- The game's own boot/error screen (`#boot`, index.html) isn't covered by the script; its error text already wraps
  (`overflow-wrap: anywhere`) and its links are inline-blocks that wrap, checked by reading only.
- Only Chromium was driven (no Firefox/Safari headless here); the CSS used (flex-wrap, gap, overflow-wrap: anywhere)
  is supported by all three.
- The page's real content with a disc imported (disc data row with a size) wasn't shown; same row markup as the others.
