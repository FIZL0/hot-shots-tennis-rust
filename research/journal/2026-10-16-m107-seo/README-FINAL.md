# M107 — SEO for the website

Goal: the M96 landing page (`web/index.html`) found for "Hot Shots Tennis", "Everybody's Tennis", "Minna no Tennis",
"Hot Shots Tennis PC / remaster / online". Name stays OpenTennis; the original's names only as what it remasters.

## What changed
- **The text is in the HTML from the start.** `#disc` (the site) used to be `hidden` until the module script had
  checked OPFS, so a crawler without scripts, or one that snapshots early, saw only "Loading…". Now it's shown by
  default; a synchronous head script sets `html[data-game]` (site.css hides `#disc`) only on the visits that go straight
  into the game (tab reload while playing / the autoplay box, and no `?site`) — the same test the module script's
  landing check makes, minus "is there a disc pack" (async); when that check still opens the site, `landing()` removes
  the attribute. Set before the body is parsed, so no flash of the site when going into the game.
- **Head**: title "OpenTennis: a Hot Shots Tennis remaster for PC, browser and online play", meta description naming
  Everybody's Tennis / Minna no Tennis, `<link rel=canonical href=https://opentennis.dev/>` (covers `?site`, `?code=`,
  and beta.opentennis.dev, which also gets `<meta name=robots content=noindex>` from a hostname check), Open Graph +
  Twitter `summary_large_image` with `https://opentennis.dev/shots/rally-autumn.jpg` (docs/screenshots, copied by
  tools/web.sh; 1280×720), JSON-LD `VideoGame` (free offer, platforms, 1–4 players, screenshots, `isBasedOn` the PS2
  game with its alternate names, Clap Hanz / SCE). No ratings (we have none; not invented).
- **Content**: the hero h1 now starts with a gold kicker "OpenTennis · a remaster of Hot Shots Tennis"; the lede names
  Everybody's Tennis / Minna no Tennis; a new "Questions" section (#faq, also in the footer nav) answers "Can I play
  Hot Shots Tennis on PC?", "Is this Everybody's Tennis or Minna no Tennis?", "…online?", "same game?", "need the
  disc?", "add characters?". No FAQPage schema: Google stopped showing FAQ rich results for sites like this (2023).
- **Fast first load**: the hero picture is preloaded (only when the site shows), the figure images have width/height
  (no layout shift). Nothing of the game (wasm) loads before Play, as before.
- `web/robots.txt` (allow all, sitemap), `web/sitemap.xml` (the one page + its images). Both deploy with web/ as is
  (pages.yml uploads the whole directory). README's title is now "OpenTennis — a remaster of Hot Shots Tennis (PS2)".
- Search Console / Bing submission, link-preview check, GitHub About/topics → plan/HUMAN.md.

## Checked
Headless Chromium on `python3 -m http.server` over web/ (screens in `context/m107/`): the site with and without
JavaScript (identical first screen), the Questions section, JSON-LD parses. Over CDP: fresh visit → site shown;
autoplay + a disc pack → `#disc` display none and the game boots; `?site` → site; autoplay without a pack → site.

## Not verified / not 1:1
- Actual ranking/indexing: can only be seen after deploy and the Search Console steps (HUMAN.md); nothing to compare
  against the original (website, not gameplay).
- Link-preview cards on Discord/X not seen live (needs the deployed site); the tags follow the OG/Twitter specs.
- The beta noindex is added by script (Google honours it after rendering); the canonical link is the main guard.
