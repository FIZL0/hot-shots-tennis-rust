# M19 — My Public Mods

Mods → Upload my mod → **My public mods** (row 2, needs the sign-in on row 1) lists the signed-in player's own
uploads to `FIZL0/opentennis-mods`; ✕ twice on one takes it down.

- `modstore::mine(name)`: reads `main`'s `mods/*/meta.json` and the open PRs (same calls as Browse, never cached),
  keeps the PRs whose author is `name` and the `main` mods whose `meta.json` author is `name` (case-insensitive).
  No merge by id: a mod both merged and on a newer PR shows twice, each removable on its own.
- Upload now writes the account's name into `meta.json` `author` (Browse already showed a PR's author for PR mods),
  so a merged upload is still found as the player's.
- `modstore::remove`: a PR mod → `POST /api/datasets/<repo>/discussions/<n>/status {"status":"closed","comment"}`
  (as `huggingface_hub.change_discussion_status`). A `main` mod → a commit PR (`commit/main?create_pr=1`) with one
  `{"key":"deletedFolder","value":{"path":"mods/<id>"}}` line (as `huggingface_hub.delete_folder`); no preupload
  when nothing is added. Both clear the Browse cache.
- Screen: `Page::Mine` in `mods_screen.rs`, rows = uploads, Refresh, Back (→ Upload's row 1). Tags/panel as Browse.
  `HST_MENU=mymods` sample; shot in `context/m19/mymods.png` (looks right).

Checks: `my_public_mods` (screen flow), existing mods-screen tests updated for the new row;
`HST_LIVE_MINE=hotshotsgaming cargo test -p hst --bin hst live_mine` against the live store listed that account's
PRs 2–4 (Anya, Wendy, Z). Tokenless POSTs to the status and commit endpoints answer 401 (they exist, need auth).

## Not verified / not 1:1
- The actual close / delete-PR against the live repo: needs a signed-in account's token, and M17 says don't write to
  the live repo unattended. Added to *Needs the human* (with M17's upload check).
- Mods merged before this change carry `mod.json`'s author, not the HF account, so they're only found if the two
  match.
- After a merged mod's delete PR is opened it disappears from the list until the next read; a Refresh shows it again
  (still on `main`) until a maintainer merges the delete.
- Front-end only, no original to compare (not in the retail game).
