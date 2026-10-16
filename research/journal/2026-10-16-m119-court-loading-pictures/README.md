# M119 — court mods' loading pictures in the mods repo

- ../opentennis-mods `standard/HST-COURT-UMPIRE-STANDARD.md` §2: `previews/loading.png` in the folder listing
  (optional, PNG, any size, ideally 16:9 ≥1280×720) and a rule: drawn full screen behind the loading panels before a
  match on the court, scaled to cover the window at its own aspect (centred, cropped, never stretched); use the
  court's own art, not a flyover screenshot (user 2026-10-16); absent/unreadable → the layout court's picture.
- New `tools/psp2gltf/export_getagrip_court_loading.py` (run after `export_getagrip_courts.py`; README pipeline line
  added): each Get a Grip court's loading panel `menu/loadmes/court/loadmes_cNN.xb0/.../CNN.gim` → its mod's
  `previews/loading.png`. The panels are clean 480×272 full-screen art (the court name is a separate layout,
  `loadmes_courtname`): nothing to cut out, unlike the umpire panels. English xb0 = French xb2 = Spanish xb3 pixel for
  pixel; the Japanese xb differs (game logo bottom-left on some courts), so xb0 is used. Written opaque RGB at native
  size (Bevy's PNG load converts to RGBA). All 16 courts exported (2.7 MB, out/ only, git-ignored).
- Mods repo commit 8180cb6 (not pushed: this unattended run may not push).
- `mods_court.rs`: the `LOADING` doc now points at the standard section; no behaviour change (M117 did the reading).

## Not verified / not 1:1
- No original to match: the original has no court mods; the loading screen is front-end (PLAN: not 1:1).
- Not checked in a real menu → match with an installed Get a Grip court mod (the cover's drawing and fallback were
  checked under M117 with a `--shot` hook); here only the exported PNGs were looked at.
- The pictures are 480×272, below the standard's "ideally ≥1280×720": that is all the art Get a Grip has; they are
  not upscaled (opentennis's upscaler covers `textures/`, not `previews/`), so they are soft on a big window.
- The mods repo commit isn't pushed, and the store's Get a Grip court packs don't carry the pictures until they are
  re-exported and re-published.
