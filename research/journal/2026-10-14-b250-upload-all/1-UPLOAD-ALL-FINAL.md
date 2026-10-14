# B250 — Upload all, re-uploads overwrite (FINAL)

## What changed
- `modstore.rs`: `publish(mods, name, token)` takes a batch and uploads it one mod at a time in one background
  task, sending `Event::Upload(dir, Up)` (Uploading → Done / Skipped / Failed) per mod; `Event::Uploaded` is gone.
  Each batch reads the account's own uploads once (`uploads_of`, now shared with My public mods) and `plan()`
  decides per mod: never uploaded → new PR; same `mod.zip` sha256 → skipped; store `meta.json` version higher
  (dotted numeric compare, "1.10" > "1.9", missing = 1) → skipped; their open PR → commit onto `refs/pr/<n>`
  (`commit(…, onto)`: preupload/commit to the PR revision, no `create_pr`); merged → a new PR whose files replace
  `mods/<id>/`. Same version with a different zip still goes up (a fix that wasn't bumped).
- `mods_screen.rs`: Upload page rows are account, My public mods, **Upload all**, the mods, Back. Upload all opens
  Upload all / Cancel; uploads no longer set `busy`, so the menu stays usable. A per-mod tag beside each row
  (Queued / Uploading / Uploaded / Skipped / Error) and a line in the hovered mod's panel (the PR URL or the
  reason); the status bar says the latest outcome. A mod already queued/uploading isn't queued again.
- Tests: `modstore::re_upload_plan` (all plan branches), `mods_screen::upload_all` (row, child, queueing, no busy,
  no double-queue, page still navigable, outcomes come back through the `page` system), `browse_and_upload_pages`
  updated for the new row.
- Shots: `HST_MENU=uploadmods` (Upload all open, tags) at 960×720 / 1280×720 / 1680×720, and `HST_CHILD=0` (first
  mod hovered, its URL line) at 4:3 and 21:9 → `context/b250/`. Layout holds at every aspect.

## Not verified / not 1:1
- Not run against the live Hub: committing onto `refs/pr/<n>` as the PR's author, and the merged-case replacement
  PR, follow `huggingface_hub`'s `create_commit(revision="refs/pr/N")` but no upload was made (M17 has never written
  to the live repo from an agent; that is the human's *Mod store upload check*). If the Hub refuses the PR commit,
  the mod's row shows Error with the Hub's message.
- A re-upload whose new package has no preview leaves the old `preview.png` in place (overwrite only, no delete).
- Front-end only, no original to match.
