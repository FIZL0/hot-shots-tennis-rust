# B433: the umpire on Get a Grip's chairs

**Cause.** A court mod plays on its layout disc court, and the umpire was drawn at that court's chair (its
`npc15` plant record). Get a Grip's stands are wherever Get a Grip puts them. c05 street plays as layout 0
(court 1), whose chair is at x = −6.82, while the street court's stand is at x = +7.82, on the other side of
the court. So the umpire sat 1.8 m up in the air on the far side, and the stand stood empty. The blue-haired figure
standing on the court side of the stand, at ground level, is one of court 1's walkers, not the umpire. Seven of the
sixteen courts had their stand on the other side from the layout's chair (c03 c05 c07 c08 c09 c16 c17), and most of
the rest were 0.3–1.3 m off. On nine courts Get a Grip also moves the chair further out for doubles.

**Data.** Get a Grip's placement file (`haichi`) has a `pc_locator` track (one, or a `__singles`/`__doubles`
pair) on every court: the umpire's seat, 3–15 cm out from the stand's origin and 1.70–1.81 m up, facing the court.
The court exporter skipped it as "informational".

**Fix.**
- Mods repo (`6876461`): `export_getagrip_courts.py` writes it to court.glb as an empty `umpire_seat` node
  (`umpire_seat__singles`/`__doubles` for the pairs). Standard §2 now describes `umpire_seat`, and §3b points to it.
  All 16 courts were re-exported into `out/mods`; byte-checked on c05: only court.glb changed, by the one node.
- `court_mod::umpire_seat`: the node for the match kind (else the plain one), in game space (`game_space` undone).
- `npcs::setup`: on a court mod with a seat, the umpire (disc or mod model) is drawn there, turned to the court
  centre like the disc's. On the other side of the court from the layout's chair she gets `OtherSide`, and
  `step`/`mod_umpire` draw her turn to the winner with the other clip (`turned`: 1 ↔ 2), so she still points at
  the winner's end. This only changes drawing: `Game::umpire` (pos, facing, side, motion) is still the layout's, so
  replays, rollback and online are unchanged. Stock courts and court mods without a seat take the old path.

**Checked.** Shots of every court's chair (`context/b433/run3/montage_singles.png`, `montage_doubles.png`):
the umpire sits on the seat on all 16 courts in singles and on all 9 doubles chairs. On c05, a Get a Grip umpire
(Natasha, `run2/c05_natasha_close.png`) and the disc's (`run2/c05_stock.png`) both sit on the street court's
chair. New test: `the_umpire_seat_is_read_in_game_space`.

## Not verified / not 1:1
- The turn to the winner on an `OtherSide` court wasn't shot at the moment of a point (it would need a point
  decided at a known tick). It follows from the turn clips being relative to the umpire's own right and left.
- How each umpire's legs fit each stand (sinking into the seat, shins through the ladder) is B429's job, not
  checked here.
- Players who installed these courts from the mod store keep the old court.glb, and their umpire stays at the
  layout's chair, until the Get a Grip courts are uploaded again from `out/mods`. That upload is not done here.
- Get a Grip's own match wasn't checked side by side (a PSP game, no capture setup). The seat is Get a Grip's own
  placement data.
- Full test run: one failure, not caused by this change. `court_mod_match::a_court_mod_is_seen_and_heard_and_never_moves_the_match`
  asserts the c05 court mod has people, which the mods repo's M112 export (02c3719, before this task) took out;
  its hash check (the court mod never moves the match) passes. Noted on M112, which owns that test.
