# B484: online mods: offer the download on the select, show the real mod on both clients

User report (2026-10-16): a peer missing the other side's mod character only ever got a stand-in, with no prompt to
download it, and the host didn't see the guest's modded character either.

## What was wrong

Tested live with two release instances on one machine (scratch dirs, each with its own `mods/`: host Ban, guest
Brad, joined by short code).

- The lobby's prompt and the download worked, but only on `Screen::Lobby`: `poll` ran `settle` only there.
  A guest joins straight onto the character select. By then the host's mod pick is already in the room, but nothing
  was fetched or asked about. The select drew the donor, and the lobby/confirm note read "{name} mod: you'll see a
  stand-in", so it looked like a stand-in with no prompt. With `online_mods = always` the looks were fetched only
  once the player was back in the lobby.
- A downloaded mod's match HUD face was its donor's face. The face is the baked preview's close-up
  (`m-<stamp>-<k>.png`), but the baker only queued on Chars/Main/Mods, so a borrowed mod was never drawn. Also,
  `Loader::path` cached stamps by mod index, which is wrong for borrowed mods: their indices change with the lobby's
  line-up.

## Fix

- `online.rs`: `poll` settles the lobby's mods on Chars and Confirm too. With "always" the looks are fetched at
  once, with "never" a stand-in is used, and with "ask" the prompt opens over the select or confirm
  (`mod_asking`/`mod_ask_step`/`draw_ask_over`).
  - The prompt dims the screen, has its own cursor (`net.ask_row`), so the select's grid cursors and the confirm's
    row stay put, and takes every device's press. ○ or a tap outside means "Stand-in this time".
  - The lobby's prompt is unchanged; its drawing is shared (`ask_popup`) and so is answering it (`answer_ask`).
- The seat's note says what happens (`remote_mod_note`): "download its looks?", "downloading its looks...", "you'll
  be asked to download it", "its looks download once picked", and "you'll see a stand-in" only once declined, set
  to never, or rejected.
- `previews.rs`: in the lobby and on the confirm, borrowed mods' picked costumes are baked, so the HUD face is the
  mod's own. Stamps are cached only for the roster's own mods.
- `legend.rs`: the prompt's legend (Move / Choose / Stand-in) on Chars/Confirm; the legend test covers both.
- `pointer.rs`: a tap on a prompt row moves `ask_row`.

## Checked

- Unit test `remote_mod_asked_on_the_select`:
  - the prompt is asked on the select without moving its cursor, and takes the press;
  - ✕ sends `Want`;
  - ○ gives a stand-in and stays on the select;
  - "always" fetches on the confirm;
  - the note text at each step.
- `tools/check.sh -p hst main_menu`: 155 passed.
- Live, guest on "ask", host on "always" (screenshots in the session scratchpad, not kept):
  - The guest gets "Ban is a mod you don't have." over the select (legend: Choose / Stand-in). "Download this one"
    → the select, the lobby and the match draw Ban, and the HUD face is Ban's.
  - The host fetches Brad: the lobby and the match draw Brad, and the HUD face is Brad's.
  - Esc on the prompt → stays on the select. The lobby shows the donor with "Ban mod: you'll see a stand-in".

## Not verified / not 1:1

- Nothing in the original to compare against: online play and mods are ours.
- Web / LAN (WebRTC) peers: packs don't travel over that transport yet (`read_packs` is iroh-only), so they still get
  the stand-in. Not tested.
- Phones (touch): the prompt's rows take taps (`Act::Row`) as the lobby's do, and a tap outside is ○. Not tried
  on a device.
- The prompt is drawn at 4:3 in the centre like the lobby's (anchor Centre, so it keeps its aspect). Only checked
  at 16:9 (1280×720).
- If the looks arrive in the last moments before the match starts, the HUD face may not be baked yet and falls back
  to the donor's. The bake takes a few frames and the lobby countdown is 3 s, so this wasn't seen.
- A borrowed mod's grade bars come from its base row (unchanged, already so before).
