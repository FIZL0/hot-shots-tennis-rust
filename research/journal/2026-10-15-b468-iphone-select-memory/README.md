# B468 — iPhone 13 web app: picking a character in Local Play reloaded the page

Report: iPhone 13 (4 GB), first Local Play, the page reloaded on picking a character, which looks like WebKit
killing the tab for memory. Measured in headless Chromium in phone layout (`tools/web_check.mjs`, MOBILE=1,
844×390 @2x), both renderers. Captures in `context/b468/` (git-ignored). Iterations: `1-measure-and-trim-FINAL.md`.
