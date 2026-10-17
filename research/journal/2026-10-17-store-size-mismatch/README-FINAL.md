# B557 — mod store "the download is 24207876 bytes, the store says 24207789"

## Finding
- The mod is `oob_pc02_sophie` (Sophie, Hot Shots Golf: Out of Bounds) on open PR 151 (uploader hotshotsgaming).
- PR 151's history: commit d3605ccd (2026-10-16 19:16Z) has Sophie **v1**, meta.json size 24207789, and its
  mod.zip is 24207789 bytes (LFS size). Commit 8c78a442 (2026-10-17 03:37Z = 23:37 EDT) re-uploaded her as **v2**:
  meta.json size 24207876, mod.zip 24207876. Each revision's meta.json matches its own zip.
- The report's card says "v1 · 24 MB" and the error names v1's size for the listing and v2's for the zip: the
  player's listing (cached up to an hour) was from before the re-upload; the install read the PR's current zip.
- The report came in at 00:17 EDT 2026-10-17. The re-read-and-retry fix (64bb47aa, 2026-10-16 23:51 EDT) is in no
  release: v1.0.0 (20:31 EDT) and web-prebuilt (18:44 EDT) both predate it. So the player's build **didn't have it**;
  this is exactly the case it handles (re-read meta.json at the same rev → sha256 differs → retry with v2).
- The store doesn't disagree with itself: the new ignored test `store_meta_matches_its_zips` (hst-base modstore)
  reads the listing the way Browse does (index + PRs) and compares every meta.json's size/sha256 with the LFS
  size/oid of the mod.zip beside it at the same rev: 103 listed zips, all match. The in-game upload already writes
  meta.json from the final zip (`package`: `size: zip.len()`, `sha256` of it) in the same commit, so no upload fix.
- Browser: `resolve/<branch>/…` answers 307 to a commit-pinned `resolve-cache/<commit>/…` URL with no
  Cache-Control, so the retry's meta.json re-read sees the new commit, not a cached old one.
- No "trust the zip if it opens" fallback added: it was conditional on the store disagreeing, which it doesn't, and
  it would weaken the sha256 check for the whole-zip path.

## Not verified / not 1:1
- Didn't reproduce the install end to end against a live re-upload (would need uploading to the store); the retry
  path itself was verified when 64bb47aa landed. Store-side consistency checked live (above).
