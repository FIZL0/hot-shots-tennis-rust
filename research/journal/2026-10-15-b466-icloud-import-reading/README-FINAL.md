# B466 — iOS iCloud ISO import: "Reading file…" before the first progress

Picking an ISO from iCloud Drive on iOS: the worker's first `FileReaderSync` read (the ISO's volume descriptor, in
`Iso::from_source`, before `write_pack_to` reports anything) blocks while iOS downloads/materialises the file, so the
bar sat empty at 0 % for a long time.

Fix (web only, no wasm change): `importer` in `web/site.js` now starts with status `Reading <name>…` and the bar in a
`busy` state (`web/site.css`: a 30 % striped segment sliding back and forth; static dimmed bar under
`prefers-reduced-motion`). The first `progress` message from the worker drops `busy` and the normal percentage
bar takes over. Error/next import reset as before (each import re-adds `busy`).

Checked: headless chromium render of the busy bar next to a normal 40 % bar (`context/B466/busy.png`).

## Not verified / not 1:1
- Not tried on a real iPhone with an iCloud-only file (no iOS device here); the cause is inferred from where the
  first progress callback fires (after the directory read).
- Front-end, ours to design: no original to match.
