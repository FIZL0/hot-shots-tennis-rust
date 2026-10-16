# M115 — Browse never goes empty while uploads finish

Every `Up::Done` used to set `View::store = None`, so Browse blanked ("Reading the mod store...") and read the
listing again after each upload of a batch: mods showed, then vanished.

Now (`play/main_menu/mods_screen.rs`):
- `View::uploaded` (the `Event::Upload` arm, moved into a method) only marks the shown listing `stale`.
- `View::refresh_stale`, run once per frame after the store's events, reads the listing again (`modstore::list(true)`,
  past the cache) when it's stale and no upload is `Queued`/`Uploading` and nothing else is reading (`busy`):
  one read per batch. `refreshing` is set while it runs.
- `View::listed` (the `Event::Listing` Ok arm) swaps the new list in and, on a refresh, puts the cursor back on the
  tile with the same `Entry::key`.
- `grid::status`: "Refreshing..." with the spinner in the footer pill while `refreshing`.
- A failed read keeps the old list (as before: `get_or_insert_default`).

Test: `uploads_keep_browse_shown` — three queued uploads (one fails) keep `tiles()` at 3 throughout, only the last
one's end starts the read, the footer says Refreshing..., the new list (two new mods sorted first) keeps the cursor
on the same mod, and no second read follows.

## Not verified / not 1:1
- Not checked against a live store upload (needs a Hugging Face token and would publish PRs): the event order is
  the one `modstore::publish` sends (Queued for all, then Uploading → Done/Skipped/Failed one at a time).
- No screenshot of the footer's "Refreshing..." pill: same pill and spinner as the downloads' status, words only.
- Front-end, not the original's (PLAN: menus are ours), so no PCSX2 comparison applies.
