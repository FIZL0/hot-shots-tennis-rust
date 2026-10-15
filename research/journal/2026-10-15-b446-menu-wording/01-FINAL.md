# B446 — menu wording pass (mods, home, online)

## The reported bug
On the Upload page, Upload all while signed out said "Sign in first: Sign in, top left." under the panel's
heading, and other places told you to sign in "with Upload (top right)", "top left" or "on the Upload tab" (a tab
that no longer exists since B383). Now every sign-in hint says the same thing in plain words:
- signed out, on My uploads: "Sign in to see your uploads: Upload (top right), then Sign in."
- Upload all / a mod while signed out: "Uploading needs a Hugging Face account: choose Sign in first."
- token expired mid-upload / mid take-down: "You've been signed out: choose Sign in, then upload again." /
  "...sign in again under Upload (top right)."
- the token steps say what the page is (Hugging Face), what to make (a token with Write access) and which
  button comes next (Paste token), without the "(Write)" shorthand.

## Other mods wording
- Remove / Remove all / Removed: "Reload afterwards to take it off the select screens" instead of
  "(Reload then takes it out of the select)"; Remove all says hand-added mods stay.
- Installed: "Reload to play it" (was "Restart the game", but the page's button is Reload).
- Reload popup line, Reload/Add mod/Search/Uploader sort/Installed/My uploads descriptions, take-down and
  ask-to-delete prompts, upload progress lines ("the menu stays yours" → "You can keep using the menu").
- Upload list chips: "Store" → "From store", a failed upload "Error" → "Failed".
- Grid footer: dropped "pictures load by themselves, cached for next time"; the per-kind online notes are
  plain sentences ("players without this mod see a stand-in character", ...).

## Home / Online
- Home: Rematch, Mods and Update (download build) descriptions; the pads caption "Connected controllers: each one takes a seat".
- Online lobby/help: computer-seat, Singles-seats, mods-allowed ("vanilla lobby", "donor in odd colours" →
  "the character it's based on, in other colours"), Ready, connecting, Find (no "BitTorrent DHT"), relay lines.
- Left as they were: the lines that were already clear (titles, buttons, Local/Settings text from the disc,
  empty states, most lobby button help).
- "Wish list" not renamed here: that's B443's.

Shots: `context/B446/run1` (before), `run2`, `run3` (after). `tools/check.sh -p hst`: 434 passed.

## Not verified / not 1:1
- The front-end is ours (PLAN: menus don't have to match the original), so nothing was compared with the original.
- Only the sample screens (signinmods, uploadmods, mymods, addmods, browsemods, main-mods) were looked at; the
  online lobby strings and the transient status lines (sign-out, failed token, etc.) were checked by reading and
  the unit tests, not by shots.
- The new strings weren't checked on a phone/portrait layout; they're about the same length as before, and
  the panels wrap text.
