# M17 — browse/share mods: store check, blocked on M16

## Store checks (2026-10-13, curl with `Origin: https://example.com`)
- `FIZL0/opentennis-mods` exists, public, holds only `.gitattributes`; no pull requests yet.
- `GET /api/datasets/FIZL0/opentennis-mods/tree/main` → 200 + `access-control-allow-origin`.
- `GET /api/datasets/FIZL0/opentennis-mods/discussions?type=pull_request&status=open` → 200 + ACAO
  (`{"discussions":[],"count":0,...}`).
- PR revisions (checked on a public dataset with open PRs, HuggingFaceFW/fineweb PR 82):
  `/api/datasets/<repo>/tree/refs%2Fpr%2F82` → 200 + ACAO; `/datasets/<repo>/resolve/refs%2Fpr%2F82/<file>` → 307
  to `/api/resolve-cache/...` → 200, ACAO on both hops. So the browser can list and download open-PR mods directly;
  `plan/M17.md`'s "pick a fallback" isn't needed.

## Why blocked
M17's install (sha256 check, then unpack into `mods/<id>/` / the OPFS store) and its Browse/Upload screens sit on
M16's zip reader, checks and OPFS store (`modfs.rs` `unpack`/`install`, `play/main_menu/mods_screen.rs`), which
were uncommitted work in slot s2 (task/M16) while this ran. Building M17 beside it would duplicate that code and
conflict at merge. Resume M17 once M16 is on main: add a store client (tree + open-PR listing, `meta.json`, hour
cache, sha256) and Browse/Upload rows on M16's mods screen, reusing `modfs` for install.

The done check (a mod uploaded from one machine shows on another) also needs a Hugging Face account to open the
upload PR: a human step, or a test account.

## Not verified / not 1:1
- Nothing implemented yet (see above). Front-end screens aren't 1:1 by rule (menus are ours).
