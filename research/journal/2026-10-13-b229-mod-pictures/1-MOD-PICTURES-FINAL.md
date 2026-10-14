# B229 — Mods page: a mod without a picture gets one drawn

The Mods page showed an installed mod's picture only if the select had already baked it (the baker ran only on the
select screen), so a mod never hovered there, or installed since the game started, said "No picture".

- `previews.rs`: `ModCard` resource. The Mods page puts its hovered mod there when it has no picture; the select's
  baker then draws that mod's costume 0 on the Mods page too (only that key: the rest of its queue waits for the
  select) with the same rig/pose/crop as the select card, into the same cache (`cache/previews/<boot>-r8/m-<stamp>-0.png`
  plus its face close-up). `baked` counts finished drawings so the page reads again. A mod not in the select's
  roster (installed this run) goes into `Loader::extra` as `Who::Mod(menu.mods.len() + n)`.
- `mods_screen.rs`: a store entry that's installed shows the installed mod's picture (its own `previews/*.png`, else
  the baked one) instead of fetching the store's `preview.png`; Upload already sends the baked card.

Checked with `--shot` (`HST_MENU=uploadmods`, a fresh folder with only Phoebe linked into `mods/`, no cache):
`context/b229/upload1.png` shows Phoebe drawn after ~8 s, and only costume 0 was baked. With the roster emptied (a
temporary hack, reverted) the same happened through the `extra` path (`context/b229/upload-extra.png`).

## Not verified / not 1:1
- Cached in the select's preview cache (beside settings.txt, keyed by the mod's folder name and file times), not in
  the mod's own folder: writing into the mod folder would change what Upload packs and what a hand-linked folder
  holds. The select shares the drawing, so nothing is drawn twice.
- A store mod that isn't installed and ships no preview still shows "No picture": drawing one from the ranged model
  fetch (download the model part only and bake it) wasn't done; it gets one once installed.
- Browser build not tried (the same baker code runs there; OPFS cache via `read`/`write`).
- Menu feature, no original to compare against (front-end is ours, PLAN.md).

## Tests
`tools/check.sh` fails only on `play::netplay::tests::matches::rollback_desync_is_saved_and_play_goes_on` ("the
desync's note", netplay.rs:861), and it fails the same way with this change reverted (already broken on main, as
B237's journal found). done.sh stops on any failing test, so the task was ticked and committed by hand.
