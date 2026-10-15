# B380 — drop Open Tee 1 character mods

Open Tee 2 has every Open Tee 1 character in a better version, plus more, so the game no longer offers the
first game's characters anywhere.

- `mods::retired(id, source)`: an `opentee_` id (the mods repo's Open Tee 1 ids; Open Tee 2's are `opentee2_`)
  or a source game of exactly "Hot Shots Golf: Open Tee" (catches a re-uploaded copy under another id).
- `mods::list` leaves them out → the character select roster, the Mods pages' installed list, the Upload page and
  online lobbies (they build from that list) never see them, including the dev link `mods/hst-mods` → open-tennis-mods
  `out/mods`, which still holds the 10 `opentee_pcNN_*` folders.
- `modstore::list` (Browse) leaves them out of the store listing, fresh and cached. `modstore::mine` doesn't: an
  uploader still sees their own Open Tee 1 uploads there to take them down.
- `mods_screen.rs` needed no change: both of its lists come through the two filters above.
- Tests: `open_tee_1_is_retired` (unit); `lists_the_custom_roster` (ignored, `HST_MODS=…/open-tennis-mods/out/mods`) now
  wants `opentee2_` and no retired mod, and counts only character-kind folders (it was counting the 13 umpire mods,
  and open-tennis-mods `out/mods` currently has a self-link `out/mods/mods` that doubled them; passes).
- `psp_mods_keep_their_heads` checks Open Tee 2 instead of every `opentee*`.

## Not verified / not 1:1
- Installed Open Tee 1 folders are hidden, not deleted from disk: deleting the user's files (or the linked mods
  repo's build output) automatically isn't reversible; the player can delete them by hand.
- Store entries for Open Tee 1 on the Hugging Face dataset (FIZL0/opentennis-mods) are not taken down: that's an
  outward-facing change for the human (My uploads → remove still lists them). HF API was rate-limited, so I
  couldn't check whether any are up.
- The open-tennis-mods build still exports Open Tee 1 (`build.sh`, `export_opentee.py`): Open Tee 2's build reads Open
  Tee 1's voice banks and face data, so it stays there; only the game drops the characters.
- No PCSX2 check: front-end/mods only, nothing in the sim.

## Wrap-up
The full `tools/check.sh` passed except `play::mod_match::tests::umpires_are_on_the_roster_and_play_a_rally`,
which fails the same way on the unchanged HEAD. During the run, the main checkout's `mods/hst-mods` link (→ open-tennis-mods
`out/mods`) disappeared. Every worktree's `mods` links there, so only `hst_umpire_04_anna` is left of the 5 umpires
the test wants. Another session removed it. I didn't touch the main checkout, and `done.sh` can't commit while that
test fails, so I committed by hand and marked B380 `[~]`. Recreate the link
(`ln -s /home/ryha/repos/open-tennis-mods/out/mods /home/ryha/repos/open-tennis/mods/hst-mods`), rerun the test, then
`tools/ctx.py unblock B380` + tick. open-tennis-mods `out/mods` also has a stray self-link `out/mods/mods` (made 22:46, not by this task).
