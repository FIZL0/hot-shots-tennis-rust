# M17: in-game upload and picker-style pages

User 2026-10-13: "the HF upload page doesn't have the files selected", "make browse and upload show like the
character picker list with a big preview and all their stats on the right half", "can we upload without opening the
huggingface page?".

## What landed
- Upload goes through the Hub API (`modstore::publish`), with no page and no outbox:
  - The player signs in once with their own *Write* token. Sign in opens the token page; the next press reads the
    clipboard (on the web, the last Ctrl+V on the page), checks it with `whoami-v2`, and keeps `<name>\n<token>` in
    `hf-token.txt` (0600, gitignored; localStorage on the web). Two presses sign out.
  - Publish: `package` (unchanged checks), then preupload (which files go to LFS), git-lfs batch (basic, or
    multipart parts + complete), verify, then an NDJSON commit to `main` with `create_pr=1`. The result is the PR
    URL, which Browse lists at once (open PRs are already in the listing).
  - Removed: `upload_url`, `hand_over`, `mods-upload/`, `web::download`, `hstDownload`.
- `Meta` gains optional `style`, `grades`, `hand`, `costumes` and `like` (the disc character it plays like). They
  are filled at upload from the select's roster (`Text.mods`), range-checked in `meta_of`, and absent in older uploads
  ("No stats").
- `preview.png`: the mod's own `previews/*.png`, otherwise the select's baked card of costume 0
  (`previews::card`), so store mods get a picture.
- Mods pages: a list on the left; on the right a panel with the picture, the △ card, the name, from/by, hand,
  costumes, plays-like, version and size, and Verified / Not reviewed / Installed / Update. The panel shows on
  Browse, on Upload and on the installed mod rows.

## Proof
- `tools/check.sh`: 502 pass. New tests: `finds_the_token`, and the Upload page's account gate in
  `browse_and_upload_pages`.
- `cargo check --target wasm32-unknown-unknown` is clean.
- Screenshots `context/m17/{browsemods,uploadmods}-2.png`: Browse with sample stats ("No picture": the
  sample has no store previews); Upload showing Phoebe's baked card and grades.

## Not verified
- No real upload has been run (it needs the user's token). Two risks in the Hub API path: Xet-backed repos (the
  LFS batch should still work for clients without Xet), and CORS on the presigned S3 PUT and its ETag in the
  browser build (huggingface.js does the same, so it should be fine).
