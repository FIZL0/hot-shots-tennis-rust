# C3c — the AI reads without the decompile (FINAL)

Same rules as C3: no behaviour change. This pass names the raw codes, bytes and magic constants in
`hst-sim/src/ai.rs`, `hst-sim/src/rally.rs` and `hst-play/src/play/doubles_ai.rs`.

## What changed

- **ai.rs**
  - `style::{NET, BASE, ALL}` (the row's style byte), `formation::{STAGGERED, ATTACKING, DEFENSIVE}`,
    `moving::{STANDING, RUNNING, STOPPED}` (the run estimate's move state), `strong::{MINUS, PLUS}` (TParam's
    hand byte as `stand_side` reads it).
  - `CHARACTERS` (14), `DOUBLES_ROWS` (84) and `RECORD_BYTES` (0x118) replace the bare numbers; `Choice::new`
    splits the setup word with `to_le_bytes` (row, forced level, strategy) instead of shifts and masks.
  - `NET_GAP` (1.5), `RUN_WIDTH`/`RUN_DEPTH` (8.685/17.885), `UNREACHABLE` (9999); the stamina constants come
    from `player::{TIRED, STAMINA_FRAMES}` (`59` is `STAMINA_FRAMES - 1`).
  - `Seen::kind` is the hitter's swing branch: compared with `shot::branch::*` (`SERVE`, `SMASH`, the
    ground-to-dive range). Its doc said "3 smash"; branch 3 is the dive and 4 the smash (play.rs's own comment
    and `reaction`'s check agree), so the doc is fixed.
  - The draws use `rng::r15`/`rng::coin`; the court widths come from `lines` (`sideline`, `BOX_MIDDLE`,
    `DOUBLES_SIDELINE`, `BASELINE`); `serve_spot`'s spots are named (`CENTRE`, `DOUBLES_MIDDLE`, the two
    one-ulp-off `*_WIDE` bit patterns kept).
  - `reach_search`'s 1.3 is `swing::STRETCH` (made `pub`, same constant).
- **rally.rs**
  - `step::{WAIT, RUN, READY, SWING}` names the receive's `state` and the NET/BASE `sub` (both 0..3).
  - `phase::{SERVE, RALLY, POINT_OVER}` (`World::phase`), `msg::*` (the AI messages 0xc/0xe/0x14/0x15/0x16/0x19),
    `guess::{NONE, WIDE, OTHER}` (the guess byte), `record::{CLEARED, NONE, HITTING}` (a shot record's −1/−2/−3),
    `CPU` (0x21) and `FIRST_CPU` for the controller test, `LOST`, `VOICE_BUSY`, `IN_REACH` (the ready check's 2).
  - Contact kinds use `aim::contact::*`, plans `aim::plan::*`; plan 11 had no name: `aim::plan::DROP` added.
  - `button` delegates to `aim::button`: the two tables were the same 19 entries. rally.rs's doc said "1 ○, 2 ×";
    aim.rs's ✕ = 1, ○ = 2 is the right one (plan 7, topspin, is ✕ and presses 1). C3b's flag is settled.
  - The from_bits literals are named: `WALK_SNAP` (0.001), `NET_BALL` (−0.91), `GRAVITY_STEP` (≈9.8/60²),
    `DIVE_LOW`/`DIVE_HIGH` (0.2/1.8), `swing::DIVE_CONE` (made `pub`, same bits), `STRETCH` (1.3), `HALF_MIDDLE`
    (2.7425), `DASH_WIDTH` (⅔ of 4.115), `LANDING_AHEAD`/`RALLY_LANDING_AHEAD` (0.5/0.3). The tier tables' 1.1 and
    1.7 are plain literals now (same bits: checked).
  - `PRESS_AT` (9), `SWING_DEFAULT` (8), `FALLBACK_ENTRY` (11); `SEEN` is `PATH_MAX` (0xb4 = 180).
  - `World::put_none` writes the records with no contact; `Rally::kind_lead` is the lead pick both set_subs had
    copied.
  - Court sides via `score::{DEUCE_COURT, AD_COURT}`, lines via `lines::*`, `path_mode == Call::Out as u8`.
- **doubles_ai.rs**: uses `phase::*`, `CPU`, `strong::*`, `guess::*`, `step::SWING`, `style::NET`, `msg::*`,
  `record::CLEARED`; the `+0x28`/`+0x3e90` offsets are gone from its comments.

## Proof

- Hash trace before any edit and after, `cmp`: identical (1 game 10703 lines, 3 games 14713 lines), files in
  `context/c3c/`.
- `tools/check.sh -p hst-sim`: 254 tests passed; `tools/check.sh -p hst-oracle --test ai`: passed.
- `cargo clippy -p hst-sim`: no new finding in these modules (the ones listed are the old negated-compare /
  collapsible-if / too-many-arguments style ones); the float lint gate in `done.sh` passes.

## Not verified / not 1:1

- Behaviour: none (rename-only; hash traces and tests bit-identical).
- Names inferred from use, not from the original: `step::*` (by what each state does), `msg::*` (from the
  existing doc), `moving::STOPPED` (the app never sets 2), `aim::plan::DROP` (plan 11: ○, short, from out wide),
  `NET_BALL`, `GRAVITY_STEP` (the value is 9.8/3600 to f32, the meaning a guess), `HALF_MIDDLE`, `VOICE_BUSY`.
- Left as integer codes with named consts, not enums (they are serialized in the snapshot and read by play.rs;
  converting them is C3f's): `Rally::state`/`sub`/`kind`/`guess`/`plan`, `World::phase`/`path_mode`,
  `Body::moving`/`strong`, `Seen::kind`.
- play.rs still calls `doubles_ai::heard(g, 0x15)` etc. with raw numbers (other agents edit play.rs): C3f can use
  `rally::msg::*` there.
- aim.rs's `OPP_SMASH = 3` compares `Look::opp_kind`, which rally.rs fills with `Seen::kind` (the branch: 3 is the
  dive). Either the name or the comparison is off; not changed here (aim.rs is C3b's, and a fix would change
  behaviour): task C31 checks it against the decompile.
