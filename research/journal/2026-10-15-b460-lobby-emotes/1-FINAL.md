# B460 — 1 (FINAL)

## What landed

- `play/main_menu/plate_emote.rs` (new): on `Screen::Lobby` / `Screen::Confirm`, each local seat's device flicks:
  a pad's right stick fully out (`emote::flick`, a new direction starts one, as `tick_input`), the keyboard's emote
  keys (`Action::EmoteUp..Right`, 1–4 by default; not while a name is typed; the chat box resets the keys itself),
  the touch pad's L1 / R1 (`emote::touch`, B271). On those screens the touch pad's L1 / R1 `just_pressed` is
  cleared before `step`, so they don't also step the costume. Nothing while a picker (court / umpire, whose L1/R1
  page), the lobby's question or its big QR covers the cards. The menus never read the right stick, so no clash.
- Rate limit as in a match: a card takes no new emote while the last one waits for its model (≤ 3 s) or plays
  (its clip's `length.ceil()` ticks at 60/s); then the card goes back to its pose (`inspect::apply(Pose)`).
- The model: `previews.rs`'s card camera for that seat (`Card(p)`, now `pub(super)`). A card still on its PNG asks
  `previews::Pending` for its live model, so the browser (which only loads a live model on a press) works too.
  `inspect::MOTIONS` now also loads `gu`/`di` (0x2c/0x2d); `inspect::react` plays any of the four (0x2e/0x2f go
  through `apply`'s Win/Loss, yaw and Brad/Michael centring included).
- Voice: the match's emote line (`sound::reaction_voice(2, motion)`: programs 9/10/7/8) from the character's singles
  voice bank set a (`audio::voice_bank`), keys alternating; a mod's own `voice/` program (key 0), a costume pack its
  disc character's, a mod without one silent.
- Net: `Cmd::Emote(seat, dir)`, `Msg::Emote { seat, dir }`, `Event::Emote` (diverted by `Net::poll` into
  `Net::emotes()`, like chat). A guest's goes to the host, which passes it on only for a seat that guest holds, a
  flick < 4, and at most one per seat per `EMOTE_GAP` (500 ms, shorter than any emote); the host relays to everyone
  else and its own app. Guests don't send or take them once playing. Never the match's input; no generator touched.
- `HST_EMOTE=0..3` (for `--shot`s): every card plays that emote as the screen opens.

## Checks

- `net::tests::emotes_in_the_lobby` (host + 2 guests over a testnet): seen by everyone else; forged seat, bad flick
  and a repeat within the gap dropped; the next after the gap passes.
- `plate_emote::tests`: keyboard flick queued for its seat, busy while waiting/playing, free after; nothing on
  another screen or under the court picker; touch pad L1 emotes and is taken from the costume step, a real pad's
  L1 isn't; the four voice lines and their alternating keys from the disc.
- `previews::tests::preview_loads_only_its_motions` updated for the two extra motions.
- Shots (`context/b460/run1/`): `HST_MENU=confirm HST_EMOTE=0|2` and `HST_MENU=lobby HST_EMOTE=3`: the cards play
  gu_set / gu / di and return to the pose (Kaito's gu is short: back by 1.2 s).

## Not verified / not 1:1

- No original to match: emotes in the menus are ours (front-end and presentation, PLAN.md: by eye only).
- Not checked live between two machines (the net path is covered by the testnet test, the drawing by shots with the
  `HST_EMOTE` hook standing in for a heard emote).
- The touch pad's L1 / R1 no longer step the costume on the lobby/confirm screens (they emote, per B271); the
  costume is still changed on the select. The lobby's help line still says "L1 / R1: costume" for pads — B449
  rewrites that strip; left to it.
- The win emote's racket can leave the top of a card's well (the inspect crop) — the select's R2 does the same.
- 0x2c / 0x2d face the camera (yaw π); only the inspect's own loss (0x2f) takes its per-character yaw.
- No legend/help hint for the emotes (hints live in B449's legend strip).
- B449 (court plates) hasn't landed: this plays on the current cards' models (the same `previews.rs` cameras the
  plates will use).
