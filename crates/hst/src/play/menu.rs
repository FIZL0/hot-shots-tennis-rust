//! The original's pause screen (textures from `AZUMA/INPANE/INPANE.XB0`), coordinates on the PS2's 640×448 screen as
//! the panel. Start opens it (system sound 2) and freezes the match: the court dimmed (`i_pause_result_15` at alpha
//! 64), the top bar (`inpane_pause03`) with "Paused" and the rules ("1 sets 4 games", `inpane_pause02`), the score
//! board at the bottom (the result board's frame, plates and each set's games, with the points from
//! `inpane_pause00`, tiebreak `01`), and the dialog (the original's `i_dialog_00`/`01`, drawn with our own box, pills
//! and words since B141): "Resume the Match" / "Return to the Menu" on pills, the chosen one bright, the hand
//! (`hsm_yubi`) beside it swinging out 16 px and back. Between them the remaster's "Settings" (B126, ours: the dialog's sheet has no such row, so `word.tm2` writes it) opens the main
//! menu's Settings screens over the paused match (`main_menu/pause.rs`); a click or tap on a pill chooses it.
//!
//! Input waits 5 ticks after it opens. Up/down move with wrap-around (repeat after 32 ticks held, then every 8;
//! sound 0, the hand starts its swing again), ✕ chooses (sound 2), Start closes it (sound 2). ○ does nothing in the
//! original; here (B340) it is Back and closes it too (sound 1), as do a right click and a click or tap off the pills.
//! "Return to the Menu" fades to black over 60 ticks; the port has no menu, so it then quits.
//! B68 (ours: the original fades out at once): once any point has been scored it first asks "Leave the match?" in
//! the same dialog, Yes / No on its pills (No chosen; up/down, ✕ chooses, ○ or Start is No, sound 1); No goes back
//! to the pause menu, Yes fades out.
//! B254 (ours): in a local match a fourth pill, "Controllers", opens the main menu's controller assignment over the
//! paused match (`main_menu/pause.rs`) to move devices between the human seats.
//! `HST_PAUSE=<seconds>` opens it after that long (for `--shot`), with `HST_PAUSE_SETTINGS=settings|graphics|controls|assign`
//! on that Settings screen (or the controller assignment), or with `HST_PAUSE_ASK=1` on the Leave the match? confirm.

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use std::sync::Arc;
use std::time::Duration;

use super::main_menu::pause::{PauseSettings, Root as PauseRoot};
use super::panel::{self, Colours};
use super::widescreen::Anchor;
use super::{Game, Pads};
use crate::Args;
use crate::audio::{Sound, SoundBank};

// Z21: not `i_dialog_00`/`01`, which our box, pills and rows (B141) stand in for, nor `hsm_yubi`, whose place the 3D
// hand always takes (`cursor::setup` at Startup)
const TEXTURES: [&str; 8] = [
    "/i_pause_result_15.tm2",
    "/inpane_pause00.tm2",
    "/inpane_pause01.tm2",
    "/inpane_pause02.tm2",
    "/inpane_pause03.tm2",
    "/result_gameset00.tm2",
    "/result_gameset01.tm2",
    "/result_gameset02.tm2",
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tex {
    /// `TEXTURES[k]`.
    Own(usize),
    /// The panel's slot labels.
    Slot,
    Face(usize),
    /// The 3D pointing hand (`cursor.rs`), where the original swung `hsm_yubi`.
    Hand,
}
const DIM: Tex = Tex::Own(0);
const POINTS: Tex = Tex::Own(1);
const TIEBREAK: Tex = Tex::Own(2);
const TEXT: Tex = Tex::Own(3);
const BAR: Tex = Tex::Own(4);
const BOARD: Tex = Tex::Own(5);
const GAMES_PAST: Tex = Tex::Own(6);
const GAMES_NOW: Tex = Tex::Own(7);
const HAND: Tex = Tex::Hand;
/// "Settings" in `word.tm2` on a row of its own (`pause::label`).
const LABEL: Tex = Tex::Own(8);
/// B141: "Resume the Match" and "Return to the Menu" set the same way, in place of `i_dialog_01`'s rows.
const RESUME: Tex = Tex::Own(9);
const RETURN: Tex = Tex::Own(10);
/// B68: the confirm's rows, set as `RESUME`: "Leave the match?", "Yes", "No".
const ASK: Tex = Tex::Own(11);
const YES: Tex = Tex::Own(12);
const NO: Tex = Tex::Own(13);
/// B254: "Controllers", set as `RESUME`.
const SEATS: Tex = Tex::Own(14);
/// B141: our pill and box layers (`menu_art::pill_layers`), in place of `i_dialog_00`'s frame and pills; after
/// [`LABELS`] in `setup` (B177: it was left at 14 when B68 added three labels, drawing the pills from "Leave the match?").
const PILL: Tex = Tex::Own(15);
/// B293: the match's court name and umpire (`pause::venue`), set as [`LABEL`], at the right of the top bar.
const COURT: Tex = Tex::Own(16);
const UMPIRE: Tex = Tex::Own(17);
/// The rows set in `setup` after `TEXTURES`, as [`LABEL`] to [`SEATS`].
const LABELS: [&str; 7] = ["Settings", "Resume the Match", "Return to the Menu", "Leave the match?", "Yes", "No", "Controllers"];
/// The dialog's colours (ours, B141): rims, its fill, a pill's fill chosen and not.
const INK: [f32; 3] = [15.0; 3];
pub(super) const PANEL: [f32; 3] = [52.0, 44.0, 36.0];
pub(super) const CHOSEN: [f32; 3] = [123.0, 105.0, 79.0];
pub(super) const UNCHOSEN: [f32; 3] = [94.0, 83.0, 70.0];
/// B171: a label row's size on its pill: 20 px of the 32-px pill high, at most 160 of its 288 px wide.
const FIT: f32 = 0.625;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Quad {
    tex: Tex,
    src: [f32; 4],
    dst: [f32; 4],
    rgb: [f32; 3],
    alpha: f32,
}

/// The dialog's rows as `i_dialog_01` numbers them: "Resume the Match", (ours) "Settings", "Return to the Menu".
const OPTIONS: [i32; 3] = [6, SETTINGS, 1];
/// Not a row of `i_dialog_01`: [`LABEL`].
const SETTINGS: i32 = -1;
/// B254: a local match's rows, with (ours) "Controllers" ([`SEATS`]) before "Return to the Menu".
const OPTIONS_LOCAL: [i32; 4] = [6, SETTINGS, CONTROLLERS, 1];
const CONTROLLERS: i32 = -2;

/// The dialog's rows: an online match has no "Controllers".
fn options(local: bool) -> &'static [i32] {
    if local { &OPTIONS_LOCAL } else { &OPTIONS }
}

/// Pill `k`'s top on the dialog of `n` rows: lifted for three sets, centred on the options.
fn pill_y(sets_to_win: i32, n: usize, k: usize) -> i32 {
    let lift = if sets_to_win == 3 { -72 } else { -32 };
    lift - (n as i32 - 1) * 16 + 0xcc + 40 * k as i32
}
/// B178: the confirm's Yes (0) / No (1) pill tops: two pills as the dialog would centre them, moved down 24 px to
/// make room for the question above.
fn ask_y(sets_to_win: i32, k: usize) -> i32 {
    pill_y(sets_to_win, 2, k) + 24
}
/// Points cells (column, row) by point index; 4 is Deuce, 5 Advantage (tiebreak: 0–7, 8 Deuce, 9 Advantage).
const POINT_U: [i32; 10] = [0, 1, 0, 1, 0, 1, 0, 1, 0, 1];
const POINT_V: [i32; 10] = [0, 0, 1, 1, 2, 2, 3, 3, 4, 4];
const WHITE: [f32; 3] = [128.0; 3];

/// The hand's swing: out to −16 slowing down, an 11-tick rest, then back speeding up (57 ticks a cycle).
#[derive(Clone, Copy, Default, Debug)]
struct Hand {
    x: f32,
    t: u32,
    stage: u32,
}

impl Hand {
    fn step(&mut self) {
        let t = self.t as f32;
        match self.stage {
            0 => {
                self.x += (t / 3.0).powi(2) / 100.0 + 0.35;
                if self.x >= 0.0 {
                    *self = Hand { x: 0.0, t: 0, stage: 1 };
                }
            }
            1 => {
                self.x -= ((30.0 - t) / 3.0).powi(2) / 100.0 + 0.35;
                if self.x <= -16.0 {
                    *self = Hand { x: -16.0, t: 0, stage: 2 };
                }
            }
            _ if self.t >= 10 => (self.stage, self.t) = (0, 0),
            _ => {}
        }
        self.t += 1;
    }
}

/// The pause state, stepped at 60 Hz.
#[derive(Resource, Default)]
struct Menu {
    open: bool,
    /// Ticks into the fade to black after "Return to the Menu".
    quitting: Option<i32>,
    /// Ticks left before input is read.
    delay: i32,
    sel: usize,
    hand: Hand,
    /// Ticks up/down held (for the repeat).
    held: [u32; 2],
    /// Presses since the last tick: Start, ✕.
    start: bool,
    confirm: bool,
    acc: f32,
    /// "Settings" chosen: `step` opens them.
    settings: bool,
    /// B254: a local match (the dialog has "Controllers"), set by `step`; "Controllers" chosen.
    local: bool,
    seats: bool,
    /// B68: a point has been scored (set by `step`), so "Return to the Menu" asks first.
    scored: bool,
    /// B68: the confirm is up, 0 = Yes, 1 = No chosen; ○ pressed since the last tick.
    ask: Option<usize>,
    cancel: bool,
}

/// B68: any point scored so far in the match.
fn scored(s: &hst_sim::score::Score) -> bool {
    s.points != [0; 2] || s.games != [0; 2] || s.sets != [0; 2] || s.set > 0
}

impl Menu {
    fn opened() -> Self {
        Menu { open: true, delay: 5, ..default() }
    }

    /// One tick of the open menu with up/down held; returns the system sound to play.
    fn tick(&mut self, up: bool, down: bool) -> Option<usize> {
        if self.quitting.is_some() {
            self.quitting = self.quitting.map(|t| t + 1);
            return None;
        }
        if let Some(a) = self.ask {
            return self.tick_ask(a, up, down);
        }
        // B340 (ours: the original's ○ does nothing here): ○ is Back, closing the menu like Start (with the back sound)
        let cancel = std::mem::take(&mut self.cancel);
        if std::mem::take(&mut self.start) {
            *self = Menu::default();
            return Some(2);
        }
        if cancel {
            *self = Menu::default();
            return Some(1);
        }
        let mut sound = None;
        if self.delay < 1 {
            // first press, then after 32 ticks held every 8; one direction at a time
            let repeat = |on: bool, n: &mut u32| {
                *n = if on { *n + 1 } else { 0 };
                *n == 1 || (*n >= 32 && (*n - 32).is_multiple_of(8))
            };
            let up = self.held[1] == 0 && repeat(up, &mut self.held[0]);
            let down = self.held[0] == 0 && repeat(down, &mut self.held[1]);
            let opts = options(self.local);
            let n = opts.len();
            if down || up {
                self.sel = if down { (self.sel + 1) % n } else { (self.sel + n - 1) % n };
                self.hand = Hand::default();
                sound = Some(0);
            } else if self.confirm {
                sound = Some(2);
                match opts[self.sel] {
                    6 => *self = Menu::default(),
                    SETTINGS => self.settings = true,
                    CONTROLLERS => self.seats = true,
                    _ if self.scored => (self.ask, self.hand) = (Some(1), Hand::default()),
                    _ => self.quitting = Some(0),
                }
            }
        } else {
            self.delay -= 1;
        }
        self.confirm = false;
        self.hand.step();
        sound
    }

    /// B68: one tick of the confirm with `a` chosen: the menu's up/down repeat over Yes/No, ✕ chooses, ○ or Start is
    /// No; a direction held keeps its count across the switch.
    fn tick_ask(&mut self, a: usize, up: bool, down: bool) -> Option<usize> {
        let (start, cancel, confirm) =
            (std::mem::take(&mut self.start), std::mem::take(&mut self.cancel), std::mem::take(&mut self.confirm));
        let repeat = |on: bool, n: &mut u32| {
            *n = if on { *n + 1 } else { 0 };
            *n == 1 || (*n >= 32 && (*n - 32).is_multiple_of(8))
        };
        let u = self.held[1] == 0 && repeat(up, &mut self.held[0]);
        let d = self.held[0] == 0 && repeat(down, &mut self.held[1]);
        let mut sound = None;
        if u || d {
            (self.ask, self.hand) = (Some(a ^ 1), Hand::default());
            sound = Some(0);
        } else if start || cancel || (confirm && a == 1) {
            (self.ask, self.hand) = (None, Hand::default());
            sound = Some(if confirm { 2 } else { 1 });
        } else if confirm {
            self.quitting = Some(0);
            sound = Some(2);
        }
        self.hand.step();
        sound
    }
}

/// What the pause screen shows.
struct View {
    players: usize,
    slots: [usize; 4],
    pill: [[u8; 3]; 4],
    /// Rules: sets to win, games a set.
    sets_to_win: i32,
    games: i32,
    points: [i32; 2],
    deuce: bool,
    advantage: bool,
    tiebreak: bool,
    /// The set in play (0-based) and each team's games per set.
    set: i32,
    set_games: [[i32; 5]; 2],
    sel: usize,
    hand_x: f32,
    fade: Option<i32>,
    /// B68: the confirm is up with Yes (0) or No (1) chosen.
    ask: Option<usize>,
    /// B254: a local match's dialog, with "Controllers".
    local: bool,
}

fn layout(v: &View) -> Vec<Quad> {
    let mut out = Vec::new();
    let mut q = |tex, src: [i32; 4], dst: [f32; 4], rgb: [f32; 3], alpha: i32| {
        out.push(Quad { tex, src: src.map(|c| c as f32), dst, rgb, alpha: alpha as f32 })
    };
    let r = |x: i32, y: i32, w: i32, h: i32| [x, y, w, h].map(|c| c as f32);
    let rgb = |c: [u8; 3]| c.map(f32::from);
    q(DIM, [0, 0, 8, 8], r(0, 0, 640, 448), WHITE, 64);
    // the top bar and the rules: best-of sets in red, games in yellow
    q(BAR, [0, 0, 16, 64], r(0, 0, 352, 64), WHITE, 128);
    q(BAR, [16, 0, 32, 64], r(352, 0, 32, 64), WHITE, 128);
    q(BAR, [48, 0, 16, 64], r(384, 0, 256, 64), WHITE, 128);
    q(TEXT, [0, 0, 64, 32], r(24, 16, 64, 32), WHITE, 128);
    q(TEXT, [(v.sets_to_win * 2 - 2) * 16, 40, 16, 24], r(176, 24, 16, 24), [127.0, 59.0, 59.0], 128);
    q(TEXT, [0, 64, 56, 24], r(192, 24, 56, 24), WHITE, 128);
    q(TEXT, [(v.games - 1) * 16, 40, 16, 24], r(248, 24, 16, 24), [127.0, 97.0, 24.0], 128);
    q(TEXT, [0, 88, 104, 24], r(264, 24, 104, 24), WHITE, 128);
    // B293 (ours): the court in the rules' yellow over the umpire, right of the rules, the court on the bar above its rim, the umpire under it
    q(COURT, [0, 0, 256, 32], r(408, 3, 192, 24), [127.0, 110.0, 40.0], 128);
    q(UMPIRE, [0, 0, 256, 32], r(424, 34, 160, 20), WHITE, 128);

    // the score board: the result board's frame and plates, each set's games, the points either side
    let short = (1..=2).contains(&v.sets_to_win);
    let (rows, edge_h, top, y0, centre_h, diag_y) =
        if short { (6, 80, 320, 328, 96, 368) } else { (10, 144, 256, 264, 160, 336) };
    let n = v.players;
    for p in 0..n {
        let (count, off) = if n < 3 { (rows, 0) } else { (rows / 2, if p > 1 { rows * 8 } else { 0 }) };
        for k in 0..count {
            q(BOARD, [1 + 16 * p as i32, 40, 14, 16], r([96, 456][p & 1], y0 + off + 16 * k, 88, 16), WHITE, 128);
        }
    }
    if n >= 3 {
        for side in 0..2 {
            q(BOARD, [0, [56, 80][side], 88, 24], r([96, 456][side], diag_y, 88, 24), WHITE, 128);
        }
    }
    let bottom = top + 16 + edge_h;
    for (u, x, w) in [(0, 88, 16), (16, 104, 432), (40, 536, 16)] {
        q(BOARD, [u, 0, 16, 16], r(x, top, w, 16), WHITE, 128);
        q(BOARD, [u, 16, 16, 8], r(x, top + 16, w, edge_h), WHITE, 128);
        q(BOARD, [u, 24, 16, 16], r(x, bottom, w, 16), WHITE, 128);
    }
    q(BOARD, [65, 41, 14, 14], r(184, y0, 272, centre_h), WHITE, 128);
    for x in [248, 390, 184, 454] {
        q(BOARD, [64, 9, 8, 22], r(x, y0, 8, centre_h), WHITE, 128);
    }
    let (y1, y2) = if short { (328, 380) } else { (272, 372) };
    let plate_y = if n < 3 { [y1 + (y2 - y1) / 2; 2] } else { [y1, y2] };
    for p in 0..n {
        let (x, y) = ([100, 496][p & 1], plate_y[p / 2]);
        q(BOARD, [80, 0, 48, 48], r(x, y, 48, 48), WHITE, 128);
        q(Tex::Face(p), [0, 0, 64, 64], r(x + 2, y + 2, 64, 64), WHITE, 128);
        q(Tex::Slot, [v.slots[p] as i32 * 40, 0, 40, 24], r([144, 456][p & 1], y + 6, 40, 24), rgb(v.pill[p]), 128);
    }
    // the set in play on the bright sheet, a past set the team didn't win at half alpha
    let (cols, games_y) = match v.sets_to_win {
        1 => (1, 360),
        2 => (3, 328),
        _ => (5, 264),
    };
    for team in 0..2 {
        for col in 0..cols {
            let y = games_y + 32 * col;
            let sheet = if col == v.set { GAMES_NOW } else { GAMES_PAST };
            if col <= v.set {
                let c = col.min(4) as usize;
                let g = v.set_games[team][c];
                let alpha = if col == v.set || v.set_games[team ^ 1][c] < g { 128 } else { 64 };
                q(sheet, [g * 32, 0, 32, 32], r([272, 336][team], y, 32, 32), WHITE, alpha);
            }
            if team == 0 {
                q(sheet, [320, 0, 24, 32], r(308, y, 24, 32), WHITE, 128);
            }
        }
    }
    // the points; with advantage the trailing team at half alpha
    let (tex, last) = if v.tiebreak { (TIEBREAK, 9) } else { (POINTS, 5) };
    for t in 0..2 {
        let p = v.points[t].clamp(0, last);
        let k = match (v.deuce, v.advantage) {
            (true, _) => last - 1,
            (false, true) if p >= last - 1 => last,
            _ => p,
        } as usize;
        let alpha = if v.advantage && k != last as usize { 64 } else { 128 };
        q(tex, [POINT_U[k] << 6, POINT_V[k] * 48, 64, 48], r([184, 390][t], if short { 352 } else { 320 }, 64, 48), WHITE, alpha);
    }

    // the dialog (B141, ours in the dialog's place and colours): a rounded box, a pill per option (bright when
    // chosen), the text, the hand; (B178) the confirm is a popup of its own there, as the main menu's Reload / Later
    // (`mods_screen.rs ask_layout`): the match dimmed, a box with the question as its title, Yes and No on pills
    let box_at = |out: &mut Vec<Quad>, frame: [f32; 4]| {
        for (i, rgb) in [INK, WHITE, PANEL].into_iter().enumerate() {
            for (src, dst) in super::menu_art::box_quads(i, frame, 16.0) {
                out.push(Quad { tex: PILL, src, dst, rgb, alpha: 128.0 });
            }
        }
    };
    let (rows, ys, sel): (Vec<Tex>, Vec<i32>, usize) = match v.ask {
        Some(a) => {
            out.push(Quad { tex: DIM, src: [0.0, 0.0, 8.0, 8.0], dst: [0.0, 0.0, 640.0, 448.0], rgb: [0.0; 3], alpha: 80.0 });
            let y0 = ask_y(v.sets_to_win, 0);
            box_at(&mut out, r(160, y0 - 46, 320, 46 + 72 + 16));
            // the title as `ask_layout`'s: 22 px high
            let (w, h) = (256.0 * 22.0 / 32.0, 22.0);
            out.push(Quad { tex: ASK, src: [0.0, 0.0, 256.0, 32.0], dst: [320.0 - w / 2.0, (y0 - 32) as f32, w, h], rgb: WHITE, alpha: 128.0 });
            (vec![YES, NO], (0..2).map(|k| ask_y(v.sets_to_win, k)).collect(), a)
        }
        None => {
            let opts = options(v.local);
            let nopt = opts.len() as i32;
            let lift = if v.sets_to_win == 3 { -72 } else { -32 };
            let top = lift - (nopt - 1) * 16 + 0xb8;
            box_at(&mut out, r(160, top, 320, 16 + 8 * (nopt * 5 - 1) + 8 + 16));
            let rows = opts
                .iter()
                .map(|&text| match text {
                    SETTINGS => LABEL,
                    CONTROLLERS => SEATS,
                    6 => RESUME,
                    _ => RETURN,
                })
                .collect();
            (rows, (0..opts.len()).map(|k| pill_y(v.sets_to_win, opts.len(), k)).collect(), v.sel)
        }
    };
    for (k, &y) in ys.iter().enumerate() {
        let fill = if k == sel { CHOSEN } else { UNCHOSEN };
        for (i, rgb) in [INK, WHITE, fill].into_iter().enumerate() {
            for (src, dst) in super::menu_art::pill_quads(i, r(176, y, 288, 32)) {
                out.push(Quad { tex: PILL, src, dst, rgb, alpha: 128.0 });
            }
        }
    }
    let mut q = |tex, src: [i32; 4], dst: [f32; 4], rgb: [f32; 3], alpha: i32| {
        out.push(Quad { tex, src: src.map(|c| c as f32), dst, rgb, alpha: alpha as f32 })
    };
    for (k, (&tex, &y)) in rows.iter().zip(&ys).enumerate() {
        let (rgb, alpha) = if k == sel { (WHITE, 128) } else { (UNCHOSEN, 64) };
        // B171: the 256×32 row's ink is as tall as the pill; drawn at `FIT` and centred it sits inside the rims
        let (w, h) = (256.0 * FIT, 32.0 * FIT);
        q(tex, [0, 0, 256, 32], [320.0 - w / 2.0, y as f32 + (32.0 - h) / 2.0, w, h], rgb, alpha);
    }
    let hy = (ys[sel] + 8) as f32;
    out.push(Quad { tex: HAND, src: [0.0, 0.0, 64.0, 32.0], dst: [v.hand_x + 128.0, hy, 64.0, 32.0], rgb: WHITE, alpha: 128.0 });
    if let Some(t) = v.fade {
        let a = (128 * t / 60).min(128) as f32;
        out.push(Quad { tex: DIM, src: [0.0, 0.0, 8.0, 8.0], dst: [0.0, 0.0, 640.0, 448.0], rgb: [0.0; 3], alpha: a });
    }
    out
}

/// The match HUD's root nodes, hidden under the pause and result screens. Not B5b's composite node (aimed at its own
/// output camera): it draws the whole frame, and hiding it leaves the clear colour (the "blue screen").
type Hud = (With<Node>, Without<ChildOf>, Without<Root>, Without<super::match_stats::Root>, Without<Slot>, Without<UiTargetCamera>, Without<PauseRoot>);

#[derive(Resource)]
struct Art(Vec<Handle<Image>>, Option<Arc<SoundBank>>);
#[derive(Component)]
struct Slot(usize);
#[derive(Component)]
struct Root;
const POOL: usize = 200;

/// The pause screen is open (online play sends it to the peers).
pub(super) fn paused(w: &World) -> bool {
    w.get_resource::<Menu>().is_some_and(|m| m.open)
}

/// A peer opened or closed the pause screen: here too (not while this one is quitting).
pub(super) fn set_paused(w: &mut World, open: bool) {
    let Some(mut m) = w.get_resource_mut::<Menu>() else { return };
    if m.open == open || m.quitting.is_some() {
        return;
    }
    *m = if open { Menu::opened() } else { Menu::default() };
    if let Some(s) = w.get_resource::<Sound>() {
        s.hold(open);
        if open {
            s.hold_followed();
        }
    }
    let mut fixed = w.resource_mut::<Time<Fixed>>();
    if open {
        fixed.set_timestep(Duration::from_secs(1 << 20));
    } else {
        fixed.set_timestep_hz(60.0);
        let over = fixed.overstep();
        fixed.discard_overstep(over);
    }
}

/// The pause screen, with the match (`main.rs` `add_match`).
pub fn plugin(app: &mut App) {
    app.init_resource::<Menu>()
        .insert_resource(super::front_end::FrontEnd { paused, set_paused, face: super::main_menu::face })
        .add_systems(PostStartup, setup.after(super::setup))
        .add_systems(Update, (step.after(super::read_input), pool, draw).chain().before(super::main_menu::pause::Systems))
        // B179: the main menus' 3D hand (`cursor.ani`) points here and on the Settings screens over the match
        .add_systems(Startup, super::main_menu::cursor::setup)
        .add_systems(Update, super::main_menu::cursor::tick.after(draw))
        .add_systems(Last, super::main_menu::cursor::show);
    super::main_menu::pause::plugin(app);
    super::main_menu::legend::plugin(app);
    app.add_systems(Update, legend.after(step));
}

/// M14: the pause menu's legend (its Settings screens fill their own, `pause.rs`): `step`'s keys and buttons.
fn legend(menu: Res<Menu>, settings: Res<PauseSettings>, last: Res<super::main_menu::legend::Last>, mut out: ResMut<super::main_menu::legend::Legend>) {
    use super::main_menu::legend::{Act, item};
    if !menu.open || settings.open || menu.quitting.is_some() {
        return;
    }
    let mut v = vec![item(Act::UpDown, "Move", &["W", "S"]), item(Act::Ok, "Choose", &["Enter", "J"])];
    if menu.ask.is_some() {
        v.extend([item(Act::Back, "No", &["Backspace", "K"]), item(Act::Start, "No", &["Esc"])]);
    } else {
        v.extend([item(Act::Back, "Resume", &["Backspace", "K"]), item(Act::Start, "Resume", &["Esc"])]);
    }
    out.one(*last, v);
}

fn setup(mut commands: Commands, args: Res<Args>, mut images: ResMut<Assets<Image>>, court_mod: Option<Res<crate::mods_court::MatchCourt>>, umpire_mod: Option<Res<crate::mods_umpire::MatchUmpire>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso.read("AZUMA/INPANE/INPANE.XB0").expect("INPANE archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let mut art = TEXTURES
        .iter()
        .map(|name| {
            let e = arc
                .entries
                .iter()
                .find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name))
                .unwrap_or_else(|| panic!("{name} in INPANE"));
            super::menu_art::image(&mut images, &arc.read(e).expect("INPANE bytes"), name)
        })
        .collect::<Vec<_>>();
    for s in LABELS {
        art.push(super::main_menu::pause::label(&mut iso, &mut images, s));
    }
    art.push(super::menu_art::pill_layers(&mut images));
    let (mut court, mut umpire) = super::main_menu::pause::venue(&mut iso, args.stage.map_or(args.court, |s| s as usize), args.umpire);
    // B401: a court mod by its own name
    if let Some(c) = &court_mod {
        court = c.0.name.clone();
    }
    // B430: an umpire mod by its own name, never the disc umpire it stands in for
    if let Some(u) = &umpire_mod {
        umpire = super::main_menu::pause::umpire_caption(&u.0.name);
    }
    art.push(super::main_menu::pause::label(&mut iso, &mut images, &court));
    art.push(super::main_menu::pause::label(&mut iso, &mut images, &umpire));
    let bank = SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd").map(Arc::new);
    commands.insert_resource(Art(art, bank));
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            GlobalZIndex(10),
            Root,
        ));
}

/// P39: the menu's quads while it's open (`pool`), none in play: UI layout walks hidden nodes too.
fn pool(mut commands: Commands, menu: Res<Menu>, root: Query<Entity, With<Root>>, slots: Query<Entity, With<Slot>>) {
    for r in &root {
        crate::ui_pool::keep(&mut commands, r, menu.open, slots, POOL, |i| (Slot(i), crate::ui_pool::quad()));
    }
}

/// Reads Start/✕/up/down (and clicks or taps on the pills), steps the menu at 60 Hz and holds the match's fixed clock
/// while it is open; with its Settings screens up, they have the input.
/// Presses made in the menu never reach the match: an emote neither (B341: the touch pad's L1/R1 tab Settings).
/// `step` runs after `read_input`, so the frame that resumes the match drops them too.
fn spent(pads: &mut Pads) {
    for s in &mut pads.slots {
        s.shot = None;
        s.serve = false;
        s.emote = None;
    }
}

fn step(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    mut fixed: ResMut<Time<Fixed>>,
    g: Res<Game>,
    mut menu: ResMut<Menu>,
    mut pads: ResMut<Pads>,
    art: Option<Res<Art>>,
    sound: Option<Res<Sound>>,
    mut exit: MessageWriter<AppExit>,
    mut auto: Local<bool>,
    (mouse, touches, window, mut settings): (Res<ButtonInput<MouseButton>>, Res<Touches>, Query<&Window, With<bevy::window::PrimaryWindow>>, ResMut<PauseSettings>),
    mut hovered: Local<Option<Vec2>>,
    net: Option<Res<super::netplay::Netplay>>,
) {
    let any = |b: GamepadButton| gamepads.iter().filter(|g| super::controls::readable(g)).any(|g| g.just_pressed(b));
    let start = keys.just_pressed(KeyCode::Escape) || any(GamepadButton::Start);
    let confirm = keys.any_just_pressed([KeyCode::Enter, KeyCode::Space, KeyCode::KeyJ]) || any(GamepadButton::South);
    let cancel = keys.any_just_pressed([KeyCode::Backspace, KeyCode::KeyK]) || any(GamepadButton::East);
    // the stick as the original's menus read it: its byte's held direction (|48| of 127, past the driver's deadzone)
    let stick = |g: &Gamepad| -hst_sim::player::pad_held(super::stick_byte(-g.left_stick().y)) as f32 + g.dpad().y;
    let up = keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) || gamepads.iter().filter(|g| super::controls::readable(g)).any(|g| stick(g) > 0.5);
    let down = keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) || gamepads.iter().filter(|g| super::controls::readable(g)).any(|g| stick(g) < -0.5);
    let play = |key: usize| {
        if let (Some(s), Some(Art(_, Some(b)))) = (&sound, art.as_deref()) {
            s.play_menu(b, key as u8);
        }
    };
    let timed = !*auto
        && std::env::var("HST_PAUSE").ok().and_then(|s| s.parse::<f32>().ok()).is_some_and(|t| time.elapsed_secs() >= t);
    if !menu.open {
        // the result screen has no pause
        if (start || timed) && g.match_stats.result.is_none() {
            *auto = true;
            *menu = Menu::opened();
            // B77: the whistle and the like stop where they are until the match goes on
            if let Some(s) = &sound {
                s.hold(true);
                s.hold_followed();
            }
            play(2);
            match std::env::var("HST_PAUSE_SETTINGS").as_deref() {
                Ok("assign") if timed => settings.open_seats(),
                Ok(_) if timed => settings.open(),
                _ => {}
            }
            if timed && std::env::var_os("HST_PAUSE_ASK").is_some() {
                menu.ask = Some(1);
            }
            // ponytail: the fixed clock runs on but never reaches a step; the overstep is thrown away on closing
            fixed.set_timestep(Duration::from_secs(1 << 20));
        }
        return;
    }
    if settings.open {
        spent(&mut pads);
        return;
    }
    let sets = g.rules.sets.clamp(1, 3);
    menu.local = net.is_none();
    let n = options(menu.local).len();
    // B178: the confirm's pills are Yes and No
    let (ask, rows) = (menu.ask.is_some(), if menu.ask.is_some() { 2 } else { n });
    let top = |k| if ask { ask_y(sets, k) } else { pill_y(sets, n, k) };
    let pill = |p: Vec2| (176.0..464.0).contains(&p.x).then(|| (0..rows).find(|&k| (0.0..32.0).contains(&(p.y - top(k) as f32)))).flatten();
    // B221: the mouse over a pill moves the hand there; a right click, or a click or tap off the pills, is Back
    let (press, right) = super::main_menu::pause::pointer_press(&window, &mouse, &touches);
    let at = super::main_menu::pause::cursor_point(&window);
    if at != *hovered
        && let Some(k) = at.and_then(pill)
    {
        match menu.ask {
            Some(a) if a != k => (menu.ask, menu.hand) = (Some(k), Hand::default()),
            None if k != menu.sel => (menu.sel, menu.hand) = (k, Hand::default()),
            _ => {}
        }
    }
    *hovered = at;
    menu.cancel |= right || press.is_some_and(|p| pill(p).is_none());
    if let Some(k) = press.and_then(pill) {
        match menu.ask {
            Some(a) => {
                if a != k {
                    (menu.ask, menu.hand) = (Some(k), Hand::default());
                }
                menu.confirm = true;
            }
            None => {
                if k != menu.sel {
                    (menu.sel, menu.hand) = (k, Hand::default());
                }
                menu.confirm = true;
            }
        }
    }
    menu.scored = scored(&g.score);
    menu.cancel |= cancel;
    menu.start |= start;
    menu.confirm |= confirm;
    menu.acc += time.delta_secs() * 60.0;
    while menu.acc >= 1.0 && menu.open {
        menu.acc -= 1.0;
        if let Some(k) = menu.tick(up, down) {
            play(k);
        }
    }
    spent(&mut pads);
    if std::mem::take(&mut menu.settings) {
        settings.open();
    }
    if std::mem::take(&mut menu.seats) {
        settings.open_seats();
    }
    if menu.quitting.is_some_and(|t| t >= 60) {
        exit.write(AppExit::Success);
    }
    if !menu.open {
        if let Some(s) = &sound {
            s.hold(false);
        }
        fixed.set_timestep_hz(60.0);
        let over = fixed.overstep();
        fixed.discard_overstep(over);
    }
}

fn draw(
    g: Res<Game>,
    pads: Res<Pads>,
    menu: Res<Menu>,
    art: Option<Res<Art>>,
    panel_art: Option<Res<panel::Art>>,
    colours: Option<Res<Colours>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut q: Query<(&Slot, &mut ImageNode, &mut Node, &mut Visibility)>,
    mut others: Query<(Entity, &mut Visibility), Hud>,
    mut hidden: Local<Vec<(Entity, Visibility)>>,
    cursor: Option<Res<super::main_menu::cursor::Cursor>>,
) {
    let (Some(art), Some(panel_art), Some(colours)) = (art, panel_art, colours) else {
        return;
    };
    let n = g.players.len();
    let s = &g.score;
    // the pause and result screens replace the rest of the HUD (panel, pop-ups, the port's text)
    let cover = menu.open || g.match_stats.result.is_some();
    if cover && hidden.is_empty() {
        for (e, mut vis) in &mut others {
            hidden.push((e, *vis));
            vis.set_if_neq(Visibility::Hidden);
        }
    } else if !cover {
        for (e, vis) in hidden.drain(..) {
            if let Ok((_, mut v)) = others.get_mut(e) {
                *v = vis;
            }
        }
    }
    let quads = if menu.open {
        layout(&View {
            players: n,
            slots: std::array::from_fn(|i| if i < n { pads.slot_of(i, n).unwrap_or(4) } else { 4 }),
            pill: colours.0,
            sets_to_win: g.rules.sets.clamp(1, 3),
            games: g.rules.games.clamp(1, 6),
            points: s.points,
            deuce: s.deuce,
            advantage: s.advantage,
            tiebreak: s.tiebreak,
            set: s.set,
            set_games: s.set_games,
            sel: menu.sel,
            // B179: the 3D hand wags in place (`cursor.ani`) as on the main menus; `hsm_yubi` swings
            hand_x: if cursor.is_some() { 0.0 } else { menu.hand.x },
            fade: menu.quitting,
            ask: menu.ask,
            local: menu.local,
        })
    } else {
        Vec::new()
    };
    let share = super::widescreen::window_share(&window);
    let aspect = super::widescreen::window_aspect(&window);
    for (Slot(i), mut img, mut node, mut vis) in &mut q {
        let Some(quad) = quads.get(*i) else {
            vis.set_if_neq(Visibility::Hidden);
            continue;
        };
        let three_d = cursor.as_ref().filter(|_| quad.tex == HAND);
        img.image = match (three_d, quad.tex) {
            (Some(c), _) => c.image.clone(),
            (_, Tex::Own(k)) => art.0[k].clone(),
            (_, Tex::Slot) => panel_art.0[1].clone(),
            (_, Tex::Face(p)) => panel_art.0[panel::FACES + p].clone(),
            (None, Tex::Hand) => {
                vis.set_if_neq(Visibility::Hidden);
                continue;
            }
        };
        let [u, v, w, h] = quad.src;
        // B160: a face is its whole image (INPANE's 64×64, or a baked one's larger square)
        img.rect = (!matches!(quad.tex, Tex::Face(_)) && three_d.is_none()).then(|| Rect::new(u, v, u + w, v + h));
        let [r, g, b] = quad.rgb.map(|c| c / 128.0);
        img.color = Color::srgba(r, g, b, quad.alpha / 128.0);
        let [mut x, mut y, mut w, mut h] = quad.dst;
        if three_d.is_some() {
            // where the main menus' rows put it (row x − 60, y + 3), not `hsm_yubi`'s (x − 48, y + 8)
            (x, y) = (x - 12.0, y - 5.0);
            // the hand's image covers `PAD` times its quad, around the same centre (B146)
            let pad = super::main_menu::cursor::PAD;
            (x, y, w, h) = (x - w * (pad - 1.0) / 2.0, y - h * (pad - 1.0) / 2.0, w * pad, h * pad);
        }
        // widescreen: the dimmer and the top bar span the window, "Paused" and the rules keep to its left
        // B293: the court and umpire pinned to the window's right, under the bar's right end
        let anchor = if [DIM, BAR, TEXT].contains(&quad.tex) {
            Anchor::Span
        } else if [COURT, UMPIRE].contains(&quad.tex) {
            Anchor::Sides
        } else {
            Anchor::Centre
        };
        let (left, width) = super::widescreen::place(x, w, share, anchor);
        let (top, height) = super::widescreen::vplace(y, h, aspect, anchor);
        crate::play::widescreen::put(&mut node, left, top, width, height);
        vis.set_if_neq(Visibility::Inherited);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> View {
        View {
            players: 4,
            slots: [0, 4, 4, 4],
            pill: [[98, 46, 61], [33, 74, 97], [105, 70, 25], [63, 78, 25]],
            sets_to_win: 1,
            games: 4,
            points: [0, 0],
            deuce: false,
            advantage: false,
            tiebreak: false,
            set: 0,
            set_games: [[0; 5]; 2],
            sel: 0,
            hand_x: 0.0,
            fade: None,
            ask: None,
            local: false,
        }
    }

    /// Slot 3's pause (doubles, one set to 4 games, 0-0): the rules digits, both points, the games and the dialog.
    /// B177: the textures' indices follow `setup`'s order: the disc sheets, the labels, the pill sheet.
    #[test]
    fn art_order() {
        let n = TEXTURES.len();
        assert_eq!([LABEL, RESUME, RETURN, ASK, YES, NO, SEATS, PILL, COURT, UMPIRE], std::array::from_fn(|i| Tex::Own(n + i)));
    }

    /// B341: a press latched while paused, an emote included, is spent before the match resumes.
    #[test]
    fn paused_presses_are_spent() {
        let mut pads = Pads::default();
        pads.slots[1] = super::super::SlotPad { shot: Some(1), serve: true, emote: Some(3), ..default() };
        spent(&mut pads);
        let s = &pads.slots[1];
        assert_eq!((s.shot, s.serve, s.emote), (None, false, None));
    }

    /// P61: the match crate reaches this screen only through `front_end`; a bare world is never paused.
    #[test]
    fn the_match_pauses_through_the_front_end() {
        use super::super::front_end;
        let mut w = World::new();
        front_end::set_paused(&mut w, true);
        assert!(!front_end::paused(&w));
        w.insert_resource(Menu::default());
        w.insert_resource(Time::<Fixed>::from_hz(60.0));
        w.insert_resource(front_end::FrontEnd { paused, set_paused, face: super::super::main_menu::face });
        front_end::set_paused(&mut w, true);
        assert!(front_end::paused(&w) && w.resource::<Menu>().open);
        // the match's clock stops while it's up
        assert_eq!(w.resource::<Time<Fixed>>().timestep(), Duration::from_secs(1 << 20));
        front_end::set_paused(&mut w, false);
        assert!(!front_end::paused(&w));
    }

    #[test]
    fn slot3_pause() {
        let qs = layout(&view());
        let at = |tex: Tex, x: f32, y: f32| qs.iter().find(|q| q.tex == tex && q.dst[0] == x && q.dst[1] == y).copied();
        assert_eq!(at(TEXT, 176.0, 24.0).unwrap().src, [0.0, 40.0, 16.0, 24.0]); // "1"
        assert_eq!(at(TEXT, 248.0, 24.0).unwrap().src, [48.0, 40.0, 16.0, 24.0]); // "4"
        assert_eq!(at(POINTS, 184.0, 352.0).unwrap().src, [0.0, 0.0, 64.0, 48.0]);
        assert!(at(POINTS, 390.0, 352.0).is_some());
        assert!(at(GAMES_NOW, 272.0, 360.0).is_some() && at(GAMES_NOW, 308.0, 360.0).is_some());
        assert!(at(Tex::Face(2), 102.0, 382.0).is_some() && at(Tex::Face(1), 498.0, 330.0).is_some());
        // three options: frame from y 120 to 272, pills at 140, 180 ("Settings", ours) and 220, the hand beside the first
        assert!(at(PILL, 160.0, 120.0).is_some() && at(PILL, 160.0, 256.0).is_some());
        assert!(at(PILL, 176.0, 140.0).is_some() && at(PILL, 176.0, 220.0).is_some());
        // the rows centred on their pills at 160×20 (B171)
        assert_eq!(at(RESUME, 240.0, 146.0).unwrap().alpha, 128.0);
        assert_eq!(at(RETURN, 240.0, 226.0).unwrap().alpha, 64.0);
        assert_eq!(at(LABEL, 240.0, 186.0).unwrap().dst, [240.0, 186.0, 160.0, 20.0]);
        assert_eq!(qs.iter().find(|q| q.tex == HAND).unwrap().dst, [128.0, 148.0, 64.0, 32.0]);
        assert_eq!((pill_y(1, 3, 1), pill_y(3, 3, 0)), (180, 100));
        // B293: the court on the bar's right end, the umpire under its rim
        assert_eq!(qs.iter().find(|q| q.tex == COURT).unwrap().dst, [408.0, 3.0, 192.0, 24.0]);
        assert_eq!(qs.iter().find(|q| q.tex == UMPIRE).unwrap().dst, [424.0, 34.0, 160.0, 20.0]);
    }

    /// B254: a local match's dialog has four pills, "Controllers" third, centred as the three; ✕ on it asks for the
    /// controller assignment.
    #[test]
    fn controllers_row() {
        let qs = layout(&View { local: true, ..view() });
        assert!(qs.iter().any(|q| q.tex == PILL && q.dst[0] == 176.0 && q.dst[1] == 124.0));
        assert_eq!(qs.iter().find(|q| q.tex == SEATS).unwrap().dst[1], 210.0);
        assert!(qs.iter().any(|q| q.tex == RETURN && q.dst[1] == 250.0));
        let mut m = Menu { local: true, ..Menu::opened() };
        for _ in 0..5 {
            m.tick(false, false);
        }
        m.tick(false, true);
        m.tick(false, false);
        m.tick(false, true);
        assert_eq!(m.sel, 2);
        m.confirm = true;
        assert_eq!(m.tick(false, false), Some(2));
        assert!(m.open && m.seats && !m.settings);
    }

    /// Advantage: "A" for the leader, the trailing 40 at half alpha; tiebreak deuce uses the red D.
    #[test]
    fn points() {
        let qs = layout(&View { points: [4, 3], advantage: true, ..view() });
        let pts: Vec<_> = qs.iter().filter(|q| q.tex == POINTS).map(|q| (q.src, q.alpha)).collect();
        assert_eq!(pts, [([64.0, 96.0, 64.0, 48.0], 128.0), ([64.0, 48.0, 64.0, 48.0], 64.0)]);
        let qs = layout(&View { points: [6, 6], deuce: true, tiebreak: true, ..view() });
        assert!(qs.iter().filter(|q| q.tex == TIEBREAK).all(|q| q.src == [0.0, 192.0, 64.0, 48.0]));
    }

    /// The HUD filter takes the panel's root but neither the frame composite nor the pause screen's own root.
    #[test]
    fn hud_roots() {
        let mut w = World::new();
        let cam = w.spawn_empty().id();
        let panel = w.spawn(Node::default()).id();
        w.spawn((Node::default(), UiTargetCamera(cam)));
        w.spawn((Node::default(), Root));
        let found: Vec<Entity> = w.query_filtered::<Entity, Hud>().iter(&w).collect();
        assert_eq!(found, [panel]);
    }

    /// Down moves and wraps, input waits 5 ticks, the hand swings to −16 and back, ✕ on the second option asks for the
    /// settings, on the third fades out.
    #[test]
    fn input() {
        let mut m = Menu::opened();
        for _ in 0..5 {
            assert_eq!(m.tick(false, true), None);
        }
        assert_eq!(m.tick(false, true), Some(0));
        assert_eq!(m.sel, 1);
        assert_eq!(m.tick(false, true), None); // held: no repeat until 32
        for _ in 0..29 {
            m.tick(false, true);
        }
        assert_eq!(m.tick(false, true), Some(0)); // 32nd tick held
        assert_eq!(m.sel, 2);
        let mut h = Hand::default();
        let xs: Vec<f32> = (0..60).map(|_| { h.step(); h.x }).collect();
        // out to −16 by tick 20, held 11 ticks, eased back to 0 at tick 58 and out again
        assert!(xs[19] > -16.0 && xs[20] == -16.0 && xs[30] == -16.0 && xs[31] > -16.0);
        assert!(xs[56] < 0.0 && xs[57] == 0.0 && xs[58] < 0.0);
        m.tick(false, false);
        m.tick(true, false);
        assert_eq!(m.sel, 1);
        m.confirm = true;
        assert_eq!(m.tick(false, false), Some(2));
        assert!(m.open && m.settings && m.quitting.is_none());
        m.sel = 2;
        m.confirm = true;
        assert_eq!(m.tick(false, false), Some(2));
        assert_eq!(m.quitting, Some(0));
        m.start = true;
        m.tick(false, false);
        assert!(m.open && m.quitting == Some(1));
        // B340: ○ resumes the match like Start, even before input is read
        let mut m = Menu::opened();
        m.cancel = true;
        assert_eq!(m.tick(false, false), Some(1));
        assert!(!m.open);
        let mut m = Menu::opened();
        m.start = true;
        assert_eq!(m.tick(false, false), Some(2));
        assert!(!m.open);
    }

    /// B68: "Return to the Menu" at 0-0 fades out at once; after a point it asks, No (chosen first, or ○) goes back to
    /// the pause menu on the same row, Yes fades out.
    #[test]
    fn confirm_return() {
        let ret = |scored: bool| {
            let mut m = Menu { sel: 2, scored, ..Menu::opened() };
            for _ in 0..5 {
                m.tick(false, false);
            }
            m.confirm = true;
            assert_eq!(m.tick(false, false), Some(2));
            m
        };
        let mut s = hst_sim::score::Score::new();
        assert!(!scored(&s));
        assert_eq!(ret(scored(&s)).quitting, Some(0));
        s.points[1] = 1;
        assert!(scored(&s));
        s.points[1] = 0;
        s.games[0] = 1;
        assert!(scored(&s));

        let mut m = ret(true);
        assert_eq!((m.ask, m.quitting), (Some(1), None));
        m.confirm = true;
        assert_eq!(m.tick(false, false), Some(2));
        assert!(m.open && m.ask.is_none() && m.quitting.is_none() && m.sel == 2);
        // ○ is No too
        let mut m = ret(true);
        m.cancel = true;
        assert_eq!(m.tick(false, false), Some(1));
        assert!(m.open && m.ask.is_none() && m.quitting.is_none());
        // up to Yes, ✕: the fade
        let mut m = ret(true);
        assert_eq!(m.tick(true, false), Some(0));
        assert_eq!(m.ask, Some(0));
        m.tick(false, false);
        m.confirm = true;
        assert_eq!(m.tick(false, false), Some(2));
        assert_eq!(m.quitting, Some(0));

        // B178: a popup of its own: the match dimmed, a box from 134 to 268, the question as its title, Yes and No on
        // pills at 180 and 220 (No bright), the hand by No; none of the pause dialog's box or pills
        let qs = layout(&View { ask: Some(1), ..view() });
        assert!(qs.iter().any(|q| q.tex == DIM && q.rgb == [0.0; 3] && q.alpha == 80.0));
        assert!(qs.iter().any(|q| q.tex == ASK && q.dst == [232.0, 148.0, 176.0, 22.0] && q.alpha == 128.0));
        assert!(qs.iter().any(|q| q.tex == PILL && q.dst[0] == 160.0 && q.dst[1] == 134.0));
        assert!(qs.iter().any(|q| q.tex == PILL && q.dst[0] == 160.0 && q.dst[1] + q.dst[3] == 268.0));
        assert!(!qs.iter().any(|q| q.tex == PILL && q.dst[1] == 120.0));
        let pills: Vec<_> = qs.iter().filter(|q| q.tex == PILL && q.dst[0] == 176.0).map(|q| (q.dst[1], q.rgb)).collect();
        assert_eq!(pills.iter().filter(|p| p.1 == UNCHOSEN).map(|p| p.0).collect::<Vec<_>>(), [180.0]);
        assert_eq!(pills.iter().filter(|p| p.1 == CHOSEN).map(|p| p.0).collect::<Vec<_>>(), [220.0]);
        assert!(qs.iter().any(|q| q.tex == YES && q.dst[1] == 186.0 && q.alpha == 64.0));
        assert!(qs.iter().any(|q| q.tex == NO && q.dst[1] == 226.0 && q.alpha == 128.0));
        assert!(!qs.iter().any(|q| [RESUME, RETURN, LABEL, SEATS].contains(&q.tex)));
        assert_eq!(qs.iter().find(|q| q.tex == HAND).unwrap().dst[1], 228.0);
    }
}
