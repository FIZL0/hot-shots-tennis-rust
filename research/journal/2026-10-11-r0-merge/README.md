# R0 — merge the open branches (2026-10-11)

`git branch -a --no-merged main` listed two branches; no slot was running on either (no `claude` process or tmux
pane in s2, no worktree for R1). Both merged into task/R0 with `git merge --no-edit`, no conflicts;
`tools/ctx.py archive` moved nothing.

- **task/B80** (slot s2, 4 commits): court models drawn as the game loads them (MODULATE → ABE, @add → no fog,
  the hole faded and first in material order, MODULATE alpha on GS integers); B80 ticked, B92 added. Real work → merged.
  The branch stays: it is s2's checked-out worktree branch, and it counts as merged once task/R0 lands on main.
- **task/R1** (no worktree, 2 commits): B56 shadow-strength fallback (light intensity ×128 when the court's row is
  0) and the R1/B63 menu→match transition capture notes, `HST_SPEED`. Real work → merged; branch deleted
  (its commits are in task/R0).
- **task/B48**: already merged into main, no worktree → deleted.
- task/B74a, task/B74b, task/B91 (s3–s5): at e327d14, already in main; slot branches, left alone.
- Pruned a stale detached worktree record under /tmp (directory gone).

## Test fix (pre-existing, not from the merge)
`mods::tests::rerigged_mod_plays_forehand_and_run` failed on main too (checked at 75c43f1 in a scratch worktree):
the local fixture `context/mods/test_pc00/model/c00.glb` dated from before M1d, so it had no `extras.noise` /
`_HST_NOISE` and the sway assertion got `None`. Rebuilt it per the M1a/M1d recipe
(`hst-gltf hst context/xb DIR` → `modding/tools/rerig.py DIR/pc00_c00.glb context/mods/test_pc00/model/c00.glb`);
`tools/check.sh` then passes all 322 tests. Only this worktree's `context/` was rebuilt; the main checkout's copy is
still the old one, so run the same two commands there.

## Not verified / not 1:1
- No gameplay or rendering changes of my own; the merged work's verification is in its own journals
  (`2026-10-11-b80-court-grass`, `2026-10-11-r1-review`).
- main's `context/mods/test_pc00` not rebuilt (not allowed to touch the main checkout from a slot).
