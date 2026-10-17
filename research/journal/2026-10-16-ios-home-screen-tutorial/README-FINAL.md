# M123 — Add to Home Screen steps on iOS Safari

The site's play section (`web/index.html` `#a2hs`) now opens with a panel on iPhone/iPad Safari in a tab: why
(Safari's address bar stays over the game; a home-screen web app runs full screen), then three steps: Share (at the
bottom on iPhone, top right on iPad, "tap ⋯ first" for iOS 26's collapsed toolbar) → Add to Home Screen (View More
if it's not there, leave Open as Web App on) → open it from the home screen. It also says the home-screen app has its own
storage, so the ISO has to be picked once more in it. Got it hides it for good (`localStorage hst-a2hs = 0`).

Detection (`web/site.js` `homeScreen`): iPhone/iPod in the UA, or an iPad (its own UA or the desktop "Macintosh" one
with touch points); Safari = `Safari/` + `Version/` and none of CriOS/FxiOS/EdgiOS/OPiOS/GSA/DuckDuckGo/Brave/…
(in-app views carry no `Safari/`); not when `navigator.standalone` or display-mode standalone/fullscreen.

Check: `node tools/web_a2hs.mjs` (headless chromium, UA + touch + `navigator.standalone` overrides): shown for iPhone
and iPad Safari, hidden for the home-screen app, Chrome/Firefox on iOS, an in-app view, Mac Safari, Android Chrome,
and after Got it (also after a reload); nothing past the screen's edge at 390 px. Screenshots in `context/M123/`.

## Not verified / not 1:1
- Not run on a real iPhone/iPad (none here): detection checked with spoofed user agents in chromium, not real
  Safari; the step wording (⋯ menu, View More, Open as Web App) follows iOS 26 Safari from memory and older iOS
  versions word it slightly differently (no Open as Web App toggle, Share on the bottom toolbar).
- No original to match: the website is ours.
