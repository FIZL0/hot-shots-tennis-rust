# B394 — American spelling "favorites"

- Every "favourite(s)" in `crates/` is now "favorite(s)": the select's tab name ("Favorites"), the legend's
  "△ Favorite", and the code (`tabs::FAVORITES`, `Favorites`, `toggle_favorite`, `Effect::SaveFavorites`, comments,
  tests) so new UI text copied from it stays American.
- The save file is now `favorites.txt`; `tabs::load` falls back to the old `favourites.txt` when the new one doesn't
  exist yet, so players keep their favorites (the next toggle writes the new name; the old file is left alone).
- `mods_screen.rs` (on the task line) had no occurrence. Plan/journal history (DONE.md, B368.md, old journals) left
  as written; B382's open line got a note to use "Favorites".
- Tests: `tools/check.sh -p hst` passed (392).

## Not verified / not 1:1
- No `--shot` of the select taken: a text-only rename, covered by the tab-name tests (`tabs::tests`).
- The `favourites.txt` fallback has no automated test (it reads beside the real `settings.txt`); checked by reading.
