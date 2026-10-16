# B491 — website GitHub pill

The hero's "Open source on GitHub" pill (`web/index.html`) now reads "GitHub" with the GitHub mark before it: an
inline 16×16 SVG path (`fill="currentColor"`, `aria-hidden`), no external request. Sizing comes from the existing
`#disc .pill svg` rule (1.2em) and the pill's inline-flex gap.

Checked with headless Chromium on a scratch page using the real `site.css` inside `.band .row`: icon and text are
centred, and the pill is the same height as "Pick your ISO" beside it.

## Not verified / not 1:1
- The full landing page wasn't rendered headless: it stops at "Opening this browser's storage…" before the hero
  shows, so the check used the hero row on its own with the real stylesheet.
