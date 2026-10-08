//! The original's score pop-up after a point (textures from `AZUMA/INPANE/INPANE.XB0`), drawn from the score show
//! that `hst_sim::flow` runs: both players' plates (pill, face, slot label, rank) beside both teams' points, the
//! scorer's old points squeezed out to the left while the new ones grow in from the right (5 steps of 25.6 px), a
//! white copy of the new points flashed on top and faded out over 15 ticks, the hold, then the whole thing fading out
//! over 5 ticks (alpha 128·t/5). At deuce it is the "Deuce!" banner instead, with the deuce count (×N) from the
//! second deuce on, squashed and restored as the show rolls. Coordinates are the PS2's 640×448 screen, as the panel.
//!
//! A tiebreak point shows the red tiebreak points instead, under the "Tie break" banner: the scorer's old points
//! slide up 3 px a tick and fade over the 4-tick swap while the new ones appear with a white flash (red "Deuce!" at
//! deuce). A game or set brings up the result board: both teams' plates, each set's games (a losing count at half
//! alpha), with sets the sets won; the scorer's new count grows to twice its size over 6 ticks while the old one
//! still shows, then shrinks back over 10 under a fading white copy. Halfway through the wait before it (tick 45 of
//! 90) "Game" / "Set" comes up over it with a white flash fading over 14 ticks, then "Server" / "Receiver" (who won
//! it, in the winning team's colour: red for the first) with its own flash over 16, held until the next point.
//!
//! The umpire's calls (Let, Out, Net, Fault, Double fault) and Change Sides are 3D models (`azuma/inpane/mdl`), each
//! played by its `.ANI`/`.MOR`/`.MTA` from frame 0 at speed 1, clamped at the end, as the effects are: a call from
//! the point's verdict while the flow's call show runs (faded out with its alpha), Change Sides through the
//! change-ends phase (80 ticks: it never reaches its 30-tick hold and fade). The model stands 10 in front of an
//! overlay camera at the origin (25° horizontal half-angle), turned π about y.

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::flow::ShowState;
use hst_sim::score::Event;

use super::panel::{self, Colours};
use super::{Game, Pads, Phase};
use crate::Args;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tex {
    Pill,
    Slot,
    Rank,
    Face(usize),
    /// Points 0/15/30/40/Deuce/Advantage, 128×64 cells (yellow).
    Points,
    /// The same cells in white (the settle flash).
    PointsWhite,
    /// "Deuce!" (256×64 at 0,0), the × and digits 0–9 in 32×32 cells below.
    Deuce,
    /// The tiebreak's red "Deuce!", same layout.
    DeuceRed,
    /// Tiebreak points 0–7 / Deuce / Advantage, 64×64 cells (128 wide for the words).
    Tiebreak,
    /// The same cells in white.
    TiebreakWhite,
    /// "Tie break", 256×64.
    TiebreakBanner,
    /// The result board's sheets `result_gameset00`–`03`: frame, stripes and pill; small digits; the current set's
    /// digits (white copies below); the sets-won digits (48×48, white copies below) and "Set".
    Board(u8),
    /// The banner's sheets `result_game`, `result_set`, `result_verBlue`, `result_verRed`: the words with their white
    /// copies a row (64) below; Server and Receiver 64 apart.
    Result(u8),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Quad {
    tex: Tex,
    src: [f32; 4],
    dst: [f32; 4],
    rgb: [f32; 3],
    alpha: f32,
}

struct View {
    players: usize,
    slots: [usize; 4],
    ranks: [Option<usize>; 4],
    pill: [[u8; 3]; 4],
    rank_rgb: [[u8; 3]; 4],
    points: [i32; 2],
    scorer: usize,
    deuce: bool,
    advantage: bool,
    deuce_count: i32,
    /// The first team's row is the top one (the original's "ends swapped" flag for the scoreboard).
    swapped: bool,
    show: ShowState,
    /// Each team's games per set, the set being played, sets won and to win, and whether the match is over.
    set_games: [[i32; 5]; 2],
    set: i32,
    sets: [i32; 2],
    sets_to_win: i32,
    match_over: bool,
    /// The board's grow and shrink steps.
    rise: i32,
    drop: i32,
}

/// Points cells (column, row) by point index; 4 is Deuce, 5 Advantage.
const POINT_U: [i32; 7] = [0, 1, 0, 1, 0, 1, 0];
const POINT_V: [i32; 7] = [0, 0, 1, 1, 2, 2, 3];

fn layout(v: &View) -> Vec<Quad> {
    let mut out = Vec::new();
    if v.players < 2 {
        return out;
    }
    match v.show.event {
        Event::Point | Event::TiebreakPoint => scores(v, &mut out),
        Event::Game | Event::Set => board(v, &mut out),
    }
    out
}

const WHITE: [f32; 3] = [128.0; 3];

fn push(out: &mut Vec<Quad>, tex: Tex, src: [f32; 4], dst: [f32; 4], rgb: [f32; 3], alpha: i32) {
    if dst[2] > 0.0 && dst[3] > 0.0 && alpha > 0 {
        out.push(Quad { tex, src, dst, rgb, alpha: alpha as f32 })
    }
}

/// The point and tiebreak-point shows.
fn scores(v: &View, out: &mut Vec<Quad>) {
    let st = v.show;
    let tiebreak = st.event == Event::TiebreakPoint;
    let mut q = |tex, src: [f32; 4], dst: [f32; 4], rgb: [f32; 3], alpha: i32| push(out, tex, src, dst, rgb, alpha);
    let rgb = |c: [u8; 3]| c.map(f32::from);
    // the fade: in over 5 ticks, out over 5 (t has already counted down this tick)
    let a = if st.fading_out {
        (128 * (st.t + 1) / 5).min(128)
    } else if st.stage == 0 {
        128 - 128 * (st.t + 1) / 5
    } else {
        128
    };

    if v.deuce {
        let deuce = if tiebreak { Tex::DeuceRed } else { Tex::Deuce };
        q(deuce, [0.0, 0.0, 256.0, 64.0], [192.0, 192.0, 256.0, 64.0], WHITE, a);
        let c = v.deuce_count;
        if c > 1 {
            let (y, h) = match st.stage {
                0 if !st.fading_out => (256 - 3 * st.t, 32),
                1 | 2 => (256 + 5 * st.n, 32 - 5 * st.n),
                _ => (256, 32),
            };
            let cell = |d: i32| if d < 8 { [d * 32, 64] } else { [(d - 8) * 32, 96] };
            let mut glyph = |[u, v]: [i32; 2], x: i32| {
                q(
                    deuce,
                    [u as f32, v as f32, 32.0, 32.0],
                    [x as f32, y as f32, 32.0, h as f32],
                    WHITE,
                    a,
                )
            };
            if c < 10 {
                glyph([64, 96], 288);
                glyph(cell(c), 320);
            } else {
                let tens = c / 10;
                glyph([64, 96], 272);
                glyph(cell(tens), if tens == 1 { 308 } else { 304 });
                glyph(cell(c % 10), 336);
            }
        }
        return;
    }

    let n = v.players;
    let s = if v.swapped { 176 } else { 0 };
    let i6 = if n < 3 { 20 } else { 0 };
    // plates: pill/face position, then slot label position
    let plates: Vec<([i32; 2], [i32; 2])> = if n < 3 {
        vec![([216, 292 - s], [248, 292 - s]), ([216, 116 + s], [248, 116 + s])]
    } else {
        vec![
            ([232, 312 - s], [264, 312 - s]),
            ([232, 136 + s], [264, 136 + s]),
            ([216, 272 + i6 - s], [248, 272 + i6 - s]),
            ([216, 96 + i6 + s], [248, 96 + i6 + s]),
        ]
    };
    let f = |p: [i32; 2], w: i32, h: i32| [p[0] as f32, p[1] as f32, w as f32, h as f32];
    for (i, &(pos, label)) in plates.iter().enumerate().take(n) {
        q(Tex::Pill, [0.0, 0.0, 112.0, 48.0], f(pos, 112, 48), rgb(v.pill[i]), a);
        q(Tex::Face(i), [0.0, 0.0, 64.0, 64.0], f(pos, 64, 64), WHITE, a);
        q(Tex::Slot, [v.slots[i] as f32 * 40.0, 0.0, 40.0, 24.0], f(label, 40, 24), rgb(v.pill[i]), a);
        if let Some(r) = v.ranks[i] {
            let at = [label[0] + 8, label[1] + 24];
            q(Tex::Rank, [0.0, r as f32 * 24.0, 64.0, 24.0], f(at, 64, 24), rgb(v.rank_rgb[i]), a);
        }
    }

    if tiebreak {
        return tiebreak_points(v, a, out);
    }
    // points: the scorer's previous value (Deuce when it just took the advantage), squeezed by the roll
    let k = v.scorer;
    let mut idx = v.points.map(|p| p.clamp(0, 6) as usize);
    idx[k] = if v.advantage { 4 } else { idx[k].saturating_sub(1) };
    let new = (idx[k] + 1).min(6);
    let cell = |i: usize| [POINT_U[i] as f32 * 128.0, POINT_V[i] as f32 * 64.0, 128.0, 64.0];
    let mut off = [128.0f32; 2];
    off[k] = match st.stage {
        0 => 128.0,
        1 if !st.fading_out => 128.0 - st.n as f32 * 25.6,
        _ => 0.0,
    };
    let y = |t: usize| 104.0 + 180.0 * ((t == 0) != v.swapped) as i32 as f32;
    for t in 0..2 {
        // with advantage the trailing team is drawn at half alpha
        let alpha = if !v.advantage || t == k {
            a
        } else if st.stage == 0 || st.fading_out {
            a / 2
        } else {
            64
        };
        q(Tex::Points, cell(idx[t]), [336.0, y(t), off[t], 64.0], WHITE, alpha);
    }
    q(Tex::Points, cell(new), [336.0 + off[k], y(k), 128.0 - off[k], 64.0], WHITE, a);
    let flash = match st.stage {
        _ if st.fading_out => 0,
        2 if st.n >= 1 => 128,
        3 => (128 * (st.t + 1) / 15).min(128),
        _ => 0,
    };
    q(Tex::PointsWhite, cell(new), [336.0, y(k), 128.0, 64.0], WHITE, flash);
}

/// Tiebreak points by index (0–7, 8 Deuce, 9 Advantage): cell column and row.
const TB_U: [i32; 11] = [0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 0];
const TB_V: [i32; 11] = [0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3];

/// The tiebreak's points, after the plates: the scorer's old points slide up 3 px a tick from the roll on and fade
/// over the swap (alpha 128·t/3) while the new ones show under a white copy (128 − 128·t/3, then 128·t/15 over the
/// settle). With advantage the words are 128 wide and the other team's points (64 wide) at half alpha.
fn tiebreak_points(v: &View, a: i32, out: &mut Vec<Quad>) {
    let st = v.show;
    let k = v.scorer;
    let mut idx = v.points.map(|p| p.clamp(0, 9) as usize);
    idx[k] = if v.advantage { 8 } else { idx[k].saturating_sub(1) };
    let new = (idx[k] + 1).min(10);
    let cell = |i: usize, w: i32| [(w * TB_U[i]) as f32, (64 * TB_V[i]) as f32, w as f32, 64.0];
    let y = |t: usize| 104.0 + 180.0 * ((t == 0) != v.swapped) as i32 as f32;
    let slide = if (1..=2).contains(&st.stage) { 3 * st.n } else { 0 };
    for t in 0..2 {
        if st.fading_out && t == k {
            continue;
        }
        let alpha = if t == k {
            match st.stage {
                2 => 128 * st.t / 3,
                3.. => 0,
                _ => a,
            }
        } else if !v.advantage {
            a
        } else if st.stage == 0 {
            a / 2
        } else {
            64
        };
        let w = if v.advantage && t == k { 128 } else { 64 };
        let up = if t == k { slide } else { 0 };
        push(out, Tex::Tiebreak, cell(idx[t], w), [336.0, y(t) - up as f32, w as f32, 64.0], WHITE, alpha);
    }
    if st.stage >= 2 {
        let w = if v.advantage { 128 } else { 64 };
        let dst = [336.0, y(k), w as f32, 64.0];
        push(out, Tex::Tiebreak, cell(new, w), dst, WHITE, a);
        let flash = match st.stage {
            _ if st.fading_out => 0,
            2 => 128 - 128 * st.t / 3,
            3 => 128 * st.t / 15,
            _ => 0,
        };
        push(out, Tex::TiebreakWhite, cell(new, w), dst, WHITE, flash);
    }
    // ponytail: the original draws it from a 256×128 rectangle; the texture is 64 tall and clamps to clear below
    push(out, Tex::TiebreakBanner, [0.0, 0.0, 256.0, 64.0], [176.0, 32.0, 256.0, 64.0], WHITE, a);
}

/// The game / set result board. `rise`/`drop`: the show's grow and shrink steps.
fn board(v: &View, out: &mut Vec<Quad>) {
    let st = v.show;
    let game = st.event == Event::Game;
    if !game && v.match_over {
        return; // the match's last set has its own finish
    }
    // the fade: in over 14 ticks, out over 5 (t has already counted down this tick)
    let a = if st.fading_out {
        (128 * (st.t + 1) / 5).min(128)
    } else if st.stage == 0 {
        128 - 128 * (st.t + 1) / 14
    } else {
        128
    };
    // phase 0: the old count, growing in the rise; 1: the new one shrinking back under its fading white copy; 2: settled
    let (phase, scale, flash) = match st.stage {
        _ if st.fading_out => (2, 1.0, 128),
        0 => (0, 1.0, 128),
        1 => (0, 1.0 + st.n as f32 / v.rise as f32, 128),
        2 => (1, 1.0 + st.n as f32 / v.drop as f32, (st.n << 7) / v.drop),
        _ => (2, 1.0, 128),
    };
    let mut q = |tex, src: [i32; 4], dst: [f32; 4], rgb: [f32; 3], alpha: i32| {
        push(out, tex, src.map(|c| c as f32), dst, rgb, alpha)
    };
    let rect = |x: i32, y: i32, w: i32, h: i32| [x as f32, y as f32, w as f32, h as f32];
    // scaled about its centre
    let grown = |x: i32, y: i32, w: i32, s: f32| {
        let (x, y, w) = (x as f32, y as f32, w as f32);
        [x + w / 2.0 - w * s / 2.0, y + w / 2.0 - w * s / 2.0, w * s, w * s]
    };
    let (x0, mid_w, centre_x, centre_w, stripe_x, plate_x, label_x) = if game {
        (152, 304, 248, 144, [160, 392], [164, 432], [208, 392])
    } else {
        (88, 432, 184, 272, [96, 456], [100, 496], [144, 456])
    };
    let short = (1..=2).contains(&v.sets_to_win);
    let (rows, edge_h, top, y0, centre_h, diag_y) =
        if short { (6, 80, 320, 328, 96, 368) } else { (10, 144, 256, 264, 160, 336) };
    let n = v.players;
    let b0 = Tex::Board(0);
    // each player's stripes behind the plates; in doubles two players a side, split by a diagonal
    for p in 0..n {
        let (count, off) = if n < 3 { (rows, 0) } else { (rows / 2, if p > 1 { rows * 8 } else { 0 }) };
        for r in 0..count {
            q(b0, [1 + 16 * p as i32, 40, 14, 16], rect(stripe_x[p & 1], y0 + off + 16 * r, 88, 16), WHITE, a);
        }
    }
    if n >= 3 {
        for side in 0..2 {
            q(b0, [0, [56, 80][side], 88, 24], rect(stripe_x[side], diag_y, 88, 24), WHITE, a);
        }
    }
    // the frame, nine pieces
    let bottom = top + 16 + edge_h;
    for (u, x, w) in [(0, x0, 16), (16, x0 + 16, mid_w), (40, x0 + 16 + mid_w, 16)] {
        q(b0, [u, 0, 16, 16], rect(x, top, w, 16), WHITE, a);
        q(b0, [u, 16, 16, 8], rect(x, top + 16, w, edge_h), WHITE, a);
        q(b0, [u, 24, 16, 16], rect(x, bottom, w, 16), WHITE, a);
    }
    q(b0, [65, 41, 14, 14], rect(centre_x, y0, centre_w, centre_h), WHITE, a);
    let bars: &[i32] = if game { &[248, 390] } else { &[248, 390, 184, 454] };
    for &x in bars {
        q(b0, [64, 9, 8, 22], rect(x, y0, 8, centre_h), WHITE, a);
    }
    // plates: pill, face, slot label in the player's colour
    let (y1, y2) = if short { (328, 380) } else { (272, 372) };
    let plate_y = if n < 3 { [y1 + (y2 - y1) / 2; 2] } else { [y1, y2] };
    for p in 0..n {
        let (x, y) = (plate_x[p & 1], plate_y[p / 2]);
        q(b0, [80, 0, 48, 48], rect(x, y, 48, 48), WHITE, a);
        q(Tex::Face(p), [0, 0, 64, 64], rect(x + 2, y + 2, 64, 64), WHITE, a);
        let tint = v.pill[p].map(f32::from);
        q(Tex::Slot, [v.slots[p] as i32 * 40, 0, 40, 24], rect(label_x[p & 1], y + 6, 40, 24), tint, a);
    }
    // each set's games; the set in play on the bright sheet, a set lost at half alpha
    let cur = if game { v.set } else { v.set - 1 };
    let (cols, games_y) = match v.sets_to_win {
        1 => (1, 360),
        2 => (3, 328),
        _ => (5, 264),
    };
    for team in 0..2 {
        for col in 0..cols {
            let y = games_y + 32 * col;
            let sheet = Tex::Board(if col == cur { 2 } else { 1 });
            if col <= cur {
                let c = col.min(4) as usize;
                let now = col == cur && team == v.scorer;
                let g = v.set_games[team][c] - (game && phase == 0 && now) as i32;
                let alpha = if col == cur || v.set_games[team ^ 1][c] < g { a } else { a / 2 };
                let s = if game && now { scale } else { 1.0 };
                q(sheet, [g * 32, 0, 32, 32], grown([272, 336][team], y, 32, s), WHITE, alpha);
                if game && phase == 1 && now {
                    q(sheet, [g * 32, 32, 32, 32], grown([272, 336][team], y, 32, s), WHITE, flash);
                }
            }
            if team == 0 {
                q(sheet, [320, 0, 24, 32], rect(308, y, 24, 32), WHITE, a);
            }
        }
    }
    // sets won, under "Set"
    if !game {
        let y = if short { 344 } else { 312 };
        for team in 0..2 {
            let x = [192, 400][team];
            let won = team == v.scorer;
            let sets = v.sets[team] - (phase == 0 && won) as i32;
            let s = if won { scale } else { 1.0 };
            q(Tex::Board(3), [288, 0, 56, 24], rect(x - 4, y + 48, 56, 24), WHITE, a);
            q(Tex::Board(3), [sets * 48, 0, 48, 48], grown(x, y, 48, s), WHITE, a);
            if phase == 1 && won {
                q(Tex::Board(3), [sets * 48, 48, 48, 48], grown(x, y, 48, s), WHITE, flash);
            }
        }
    }
    // the original queues each sheet and draws them in turn
    out.sort_by_key(|q| match q.tex {
        Tex::Board(k) => k,
        Tex::Face(_) => 4,
        _ => 5,
    });
}

/// The sheets the panel doesn't load, in `ART` order.
#[derive(Resource)]
struct Art([Handle<Image>; 13]);
const ART: [&str; 13] = [
    "/inpane_kihontokuten01.tm2",
    "/inpane_duce00.tm2",
    "/inpane_duce01.tm2",
    "/inpane_tiebreak01.tm2",
    "/inpane_tiebreak02.tm2",
    "/result_gameset00.tm2",
    "/result_gameset01.tm2",
    "/result_gameset02.tm2",
    "/result_gameset03.tm2",
    "/result_game.tm2",
    "/result_set.tm2",
    "/result_verblue.tm2",
    "/result_verred.tm2",
];
#[derive(Component)]
struct Slot(usize);

const POOL: usize = 80;

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, (setup.after(super::setup), setup_calls))
        .add_systems(PostStartup, setup_banners.after(setup))
        .add_systems(Update, (draw, draw_calls, draw_banners))
        .add_systems(FixedUpdate, (tick_calls, tick_banners, tick_result).after(super::simulate))
        .init_resource::<ResultBanner>();
}

/// The call models in the scoreboard's order, and each judge's call's model (1 out, 2 fault, 3 double fault, 4 let,
/// 5 out after net: Net).
const CALL_MODELS: [&str; 6] = ["i_let_00", "i_out_00", "i_net_00", "i_fault_00", "i_doublefault_00", "i_coatchange_00"];
const CALL_MODEL: [usize; 6] = [0, 1, 3, 4, 0, 2];
const CHANGE_SIDES: usize = 5;
/// The overlay camera's render layer: it sees only the call models.
const CALL_LAYER: usize = 7;
/// The overlay camera's horizontal half-angle (its 50° field of view), the model's distance in front of it and the
/// uniform scale the scoreboard gives each call model when it loads them.
const CALL_FOV_DEG: f32 = 25.0;
const CALL_DISTANCE: f32 = 10.0;
const CALL_SCALE: f32 = 5.7;

/// The camera drawing the call models over the match.
#[derive(Component)]
struct CallCamera;

#[derive(Resource)]
struct Calls {
    models: Vec<(hst_sim::effect::Effect, crate::effects::Shown)>,
    playing: Option<usize>,
    alpha: f32,
}

#[allow(clippy::too_many_arguments)]
fn setup_calls(
    mut commands: Commands,
    args: Res<Args>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut bindposes: ResMut<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>,
) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso.read("AZUMA/INPANE/INPANE.XB0").expect("INPANE archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let layer = bevy::camera::visibility::RenderLayers::layer(CALL_LAYER);
    // game space (y down) → Bevy, as the court's root
    let root = commands.spawn((Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::PI)), Visibility::default())).id();
    let models = CALL_MODELS.map(|name| {
        let (mut effect, shown) = crate::effects::model(&arc, &format!("azuma/inpane/mdl/{name}"), &mut commands, root, &mut meshes, &mut materials, &mut images, &mut bindposes)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        effect.hold = true;
        commands.entity(shown.root).insert(Transform::from_xyz(0.0, 0.0, CALL_DISTANCE).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)).with_scale(Vec3::splat(CALL_SCALE)));
        (effect, shown)
    });
    commands.entity(root).insert_recursive::<Children>(layer.clone());
    commands.spawn((
        CallCamera,
        Camera3d::default(),
        Camera { order: 1, clear_color: ClearColorConfig::None, ..default() },
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Projection::Perspective(PerspectiveProjection::default()),
        Transform::default(),
        layer,
    ));
    commands.insert_resource(Calls { models: models.into(), playing: None, alpha: 1.0 });
}

/// After the match's tick: start the model of a call just made (its animation's length to the flow's settle) or of
/// the change-ends phase just entered, else play the one on show a frame.
fn tick_calls(mut g: ResMut<Game>, calls: Option<ResMut<Calls>>) {
    let Some(mut calls) = calls else { return };
    let (model, start, alpha) = match (&g.phase, g.post.as_ref().and_then(|p| p.call().map(|c| (c, p.tick == 0)))) {
        (Phase::ChangeEnds(n), _) => (Some(CHANGE_SIDES), *n == hst_sim::flow::CHANGE_ENDS, 1.0),
        (_, Some((c, fresh))) => (Some(CALL_MODEL[c.call as usize]), fresh, c.alpha),
        _ => (None, false, 1.0),
    };
    calls.alpha = alpha;
    let Some(k) = model else { return calls.playing = None };
    let restart = start || calls.playing != Some(k);
    let effect = &mut calls.models[k].0;
    if restart {
        effect.start();
        if let Some(p) = g.post.as_mut() {
            p.set_call_anim(effect.clip.length);
        }
    } else {
        effect.tick();
    }
    calls.playing = Some(k);
}

/// Pose, morph and fade the model on show (hide the others); the overlay's projection follows the window as the
/// match camera's does.
fn draw_calls(
    calls: Option<Res<Calls>>,
    window: Query<&Window>,
    mut cam: Query<&mut Projection, With<CallCamera>>,
    mut q: Query<(&mut Visibility, &mut bevy::mesh::morph::MorphWeights)>,
    mut joints: Query<&mut Transform>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(calls) = calls else { return };
    if let Ok(mut proj) = cam.single_mut()
        && let Projection::Perspective(p) = &mut *proj
    {
        let aspect = window.single().map_or(4.0 / 3.0, |w| w.width() / w.height().max(1.0));
        p.fov = 2.0 * (CALL_FOV_DEG.to_radians().tan() * super::SHOWN_ASPECT.max(1.0 / aspect)).atan();
    }
    for (k, (effect, view)) in calls.models.iter().enumerate() {
        if calls.playing == Some(k) {
            crate::effects::pose(effect, view, &mut q, &mut joints, &mut materials);
            // the scoreboard's fade scales every material's alpha
            for (h, a) in view.materials.iter().zip(&effect.alphas) {
                if let Some(mut m) = materials.get_mut(h) {
                    m.base_color.set_alpha(a * calls.alpha);
                }
            }
        } else if let Ok((mut v, _)) = q.get_mut(view.root) {
            *v = Visibility::Hidden;
        }
    }
}

fn setup(mut commands: Commands, args: Res<Args>, mut images: ResMut<Assets<Image>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso.read("AZUMA/INPANE/INPANE.XB0").expect("INPANE archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let art = ART.map(|name| {
        let e = arc
            .entries
            .iter()
            .find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name))
            .unwrap_or_else(|| panic!("{name} in INPANE"));
        panel::image(&mut images, &arc.read(e).expect("INPANE bytes"))
    });
    commands.insert_resource(Art(art));
    commands
        .spawn(Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() })
        .with_children(|p| {
            for i in 0..POOL {
                p.spawn((Slot(i), ImageNode { image_mode: NodeImageMode::Stretch, ..default() }, Node { position_type: PositionType::Absolute, ..default() }, Visibility::Hidden));
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn draw(
    g: Res<Game>,
    pads: Res<Pads>,
    art: Option<Res<Art>>,
    panel_art: Option<Res<panel::Art>>,
    colours: Option<Res<Colours>>,
    cam: Query<&Transform, With<crate::Orbit>>,
    mut first_near: Local<Option<bool>>,
    result: Res<ResultBanner>,
    mut q: Query<(&Slot, &mut ImageNode, &mut Node, &mut Visibility)>,
) {
    let (Some(art), Some(panel_art), Some(colours)) = (art, panel_art, colours) else {
        return;
    };
    let n = g.players.len();
    // ponytail: the original fixes its row order at the serve from the match camera; we latch the panel's
    // camera-side test during the serve (the post-point cut-aways may look from the other end)
    if matches!(g.phase, Phase::Serve) || first_near.is_none() {
        *first_near = Some(cam.single().map_or(true, |c| c.translation.z * g.players[0].end >= 0.0));
    }
    let mut quads = match g.post.as_ref().and_then(|p| p.show()) {
        Some(show) => {
            let slot = |i: usize| pads.slot_of(i, n).unwrap_or(4);
            layout(&View {
                players: n,
                slots: std::array::from_fn(|i| if i < n { slot(i) } else { 4 }),
                // ponytail: COM rank row 0, as the panel
                ranks: std::array::from_fn(|i| (i < n && slot(i) == 4).then_some(0)),
                pill: colours.0,
                rank_rgb: colours.1,
                points: g.score.points,
                scorer: g.post_winner.clamp(0, 1) as usize,
                deuce: g.score.deuce,
                advantage: g.score.advantage,
                deuce_count: g.score.deuce_count,
                swapped: !first_near.unwrap_or(true),
                show,
                set_games: g.score.set_games,
                set: g.score.set,
                sets: g.score.sets,
                sets_to_win: g.rules.sets,
                match_over: g.score.match_over,
                rise: g.board.game_rise,
                drop: g.board.game_drop,
            })
        }
        None => Vec::new(),
    };
    if let Some(b) = &result.0 {
        quads.extend(b.quads());
    }
    for (Slot(i), mut img, mut node, mut vis) in &mut q {
        let Some(quad) = quads.get(*i) else {
            *vis = Visibility::Hidden;
            continue;
        };
        // the panel's texture order: pill, slot, …, points (3), …, rank (6), …, then the faces
        let handle = match quad.tex {
            Tex::Pill => &panel_art.0[0],
            Tex::Slot => &panel_art.0[1],
            Tex::Points => &panel_art.0[3],
            Tex::Rank => &panel_art.0[6],
            Tex::Face(p) => &panel_art.0[panel::FACES + p],
            Tex::PointsWhite => &art.0[0],
            Tex::Deuce => &art.0[1],
            Tex::DeuceRed => &art.0[2],
            Tex::Tiebreak => &panel_art.0[4],
            Tex::TiebreakWhite => &art.0[3],
            Tex::TiebreakBanner => &art.0[4],
            Tex::Board(k) => &art.0[5 + k as usize],
            Tex::Result(k) => &art.0[9 + k as usize],
        };
        let [u, v, w, h] = quad.src;
        img.image = handle.clone();
        img.rect = Some(Rect::new(u, v, u + w, v + h));
        let [r, g, b] = quad.rgb.map(|c| c / 128.0);
        img.color = Color::srgba(r, g, b, quad.alpha / 128.0);
        let [x, y, w, h] = quad.dst;
        node.left = Val::Percent(x / 6.4);
        node.top = Val::Percent(y / 4.48);
        node.width = Val::Percent(w / 6.4);
        node.height = Val::Percent(h / 4.48);
        *vis = Visibility::Inherited;
    }
}

/// The wait tick that brings up the "Game" / "Set" banner, and its two flashes' lengths (the second is the board's
/// grow plus shrink, 6 + 10).
const RESULT_AT: i32 = 45;
const RESULT_FLASH: [i32; 2] = [14, 16];

/// The "Game" / "Set" + "Server" / "Receiver" banner over the result board.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Outcome {
    set: bool,
    /// The receiving team won it.
    receiver: bool,
    winner: usize,
    /// 0 the first word flashing, 1 the second, 2 held; `t` the flash's countdown.
    stage: u8,
    t: i32,
}

#[derive(Resource, Default)]
struct ResultBanner(Option<Outcome>);

impl Outcome {
    fn new(set: bool, server: i32, winner: usize) -> Self {
        let server_team = (server != 0 && server != 2) as usize;
        Outcome { set, receiver: server_team != winner, winner, stage: 0, t: RESULT_FLASH[0] }
    }

    fn step(&mut self) {
        if self.stage < 2 {
            self.t -= 1;
            if self.t < 0 {
                self.t = if self.stage == 0 { RESULT_FLASH[1] } else { 0 };
                self.stage += 1;
            }
        }
    }

    fn quads(&self) -> Vec<Quad> {
        let mut out = Vec::new();
        let r = self.receiver as i32;
        let (x1, x2) = if self.receiver { (124.0, 276.0) } else { (148.0, 300.0) };
        let word = Tex::Result(self.set as u8);
        push(&mut out, word, [0.0, 0.0, 144.0, 64.0], [x1, 32.0, 144.0, 64.0], WHITE, 128);
        if self.stage == 0 {
            push(&mut out, word, [0.0, 64.0, 144.0, 64.0], [x1, 32.0, 144.0, 64.0], WHITE, self.t * 128 / RESULT_FLASH[0]);
        } else {
            let who = Tex::Result(if self.winner == 0 { 3 } else { 2 });
            let dst = [x2, 32.0, 256.0, 64.0];
            push(&mut out, who, [0.0, (r * 64) as f32, 256.0, 64.0], dst, WHITE, 128);
            push(&mut out, who, [0.0, ((r + 2) * 64) as f32, 256.0, 64.0], dst, WHITE, self.t.max(0) * 128 / RESULT_FLASH[1]);
        }
        out
    }
}

/// After the match's tick: bring the banner up at the game / set wait's tick 45 (not on the match's last point),
/// step it, and drop it with the post-point phase.
fn tick_result(g: Res<Game>, mut b: ResMut<ResultBanner>) {
    let Some(post) = g.post.as_ref() else { return b.0 = None };
    if post.waited() == Some(RESULT_AT)
        && let Some(e @ (Event::Game | Event::Set)) = post.event
        && !g.score.match_over
    {
        b.0 = Some(Outcome::new(e == Event::Set, g.score.server, g.post_winner.clamp(0, 1) as usize));
    }
    if let Some(r) = &mut b.0 {
        r.step();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn result_banner() {
        // server player 1 (second team) wins the set: "Set" at 148, then blue "Server" at 300
        let mut b = super::Outcome::new(true, 1, 1);
        b.step();
        let q = b.quads();
        assert_eq!((q[0].tex, q[0].dst[0], q[1].src[1], q[1].alpha), (super::Tex::Result(1), 148.0, 64.0, 118.0));
        for _ in 0..13 {
            b.step();
        }
        assert_eq!((b.stage, b.t, b.quads().len()), (0, 0, 1));
        b.step();
        let q = b.quads();
        assert_eq!(q.len(), 3);
        assert_eq!((q[1].tex, q[1].src, q[1].dst[0], q[2].src[1], q[2].alpha), (super::Tex::Result(2), [0.0, 0.0, 256.0, 64.0], 300.0, 128.0, 128.0));
        for _ in 0..17 {
            b.step();
        }
        assert_eq!((b.stage, b.quads().len()), (2, 2));
        // the receiving first team wins a game: red "Receiver" at 276
        let mut b = super::Outcome::new(false, 1, 0);
        for _ in 0..15 {
            b.step();
        }
        let q = b.quads();
        assert_eq!((q[0].tex, q[0].dst[0], q[1].tex, q[1].src[1], q[1].dst[0], q[2].src[1]), (super::Tex::Result(0), 124.0, super::Tex::Result(3), 64.0, 276.0, 192.0));
    }

    use super::*;

    fn singles(stage: u8, t: i32, n: i32, fading_out: bool) -> View {
        View {
            players: 2,
            slots: [0, 4, 4, 4],
            ranks: [None, Some(0), None, None],
            pill: [[98, 46, 61], [33, 74, 97], [0; 3], [0; 3]],
            rank_rgb: [[0; 3]; 4],
            points: [2, 0],
            scorer: 0,
            deuce: false,
            advantage: false,
            deuce_count: 0,
            swapped: false,
            show: ShowState { event: Event::Point, stage, t, n, fading_out },
            set_games: [[0; 5]; 2],
            set: 0,
            sets: [0; 2],
            sets_to_win: 1,
            match_over: false,
            rise: 6,
            drop: 10,
        }
    }

    fn points(quads: &[Quad]) -> Vec<(Tex, [i32; 4], [i32; 4], i32)> {
        quads
            .iter()
            .filter(|q| matches!(q.tex, Tex::Points | Tex::PointsWhite))
            .map(|q| (q.tex, q.src.map(|s| s as i32), q.dst.map(|s| s as i32), q.alpha as i32))
            .collect()
    }

    /// 15-0 → 30-0 mid-roll (step 2 of 5): the old 15 squeezed to 76.8 px at x 336, the new 30 from 412.8.
    #[test]
    fn roll() {
        let quads = layout(&singles(1, -1, 2, false));
        assert_eq!(
            points(&quads),
            [
                (Tex::Points, [128, 0, 128, 64], [336, 284, 76, 64], 128),
                (Tex::Points, [0, 0, 128, 64], [336, 104, 128, 64], 128),
                (Tex::Points, [0, 64, 128, 64], [412, 284, 51, 64], 128),
            ]
        );
        let pills: Vec<_> = quads.iter().filter(|q| q.tex == Tex::Pill).map(|q| [q.dst[0] as i32, q.dst[1] as i32]).collect();
        assert_eq!(pills, [[216, 292], [216, 116]]);
    }

    /// The fade in, the flash and its settle, and the fade out, by alpha.
    #[test]
    fn alphas() {
        let a = |v: View| layout(&v).iter().find(|q| q.tex == Tex::Pill).map(|q| q.alpha as i32);
        assert_eq!((0..5).rev().map(|t| a(singles(0, t, 0, false))).collect::<Vec<_>>(), [None, Some(26), Some(52), Some(77), Some(103)]);
        assert_eq!(a(singles(4, 5, 0, true)), Some(128));
        assert_eq!(a(singles(4, 0, 0, true)), Some(25));
        let flash = |v: View| points(&layout(&v)).iter().find(|p| p.0 == Tex::PointsWhite).map(|p| p.3);
        assert_eq!(flash(singles(2, -1, 0, false)), None);
        assert_eq!(flash(singles(2, -1, 1, false)), Some(128));
        assert_eq!(flash(singles(3, 15, 5, false)), Some(128));
        assert_eq!(flash(singles(3, 7, 5, false)), Some(68));
        assert_eq!(flash(singles(3, -1, 5, false)), None);
    }

    /// Taking the advantage: the scorer rolls Deuce → Advantage, the other team's 40 at half alpha.
    #[test]
    fn advantage() {
        let mut v = singles(1, -1, 0, false);
        (v.points, v.scorer, v.advantage) = ([3, 4], 1, true);
        assert_eq!(
            points(&layout(&v)),
            [
                (Tex::Points, [128, 64, 128, 64], [336, 284, 128, 64], 64),
                (Tex::Points, [0, 128, 128, 64], [336, 104, 128, 64], 128),
            ]
        );
    }

    #[test]
    fn deuce() {
        let mut v = singles(1, -1, 2, false);
        (v.points, v.deuce, v.deuce_count) = ([3, 3], true, 12);
        let quads = layout(&v);
        let at: Vec<_> = quads.iter().map(|q| (q.src.map(|s| s as i32), q.dst.map(|s| s as i32))).collect();
        assert_eq!(
            at,
            [
                ([0, 0, 256, 64], [192, 192, 256, 64]),
                ([64, 96, 32, 32], [272, 266, 32, 22]),
                ([32, 64, 32, 32], [308, 266, 32, 22]),
                ([64, 64, 32, 32], [336, 266, 32, 22]),
            ]
        );
    }

    fn quads(v: &View, f: impl Fn(Tex) -> bool) -> Vec<(Tex, [i32; 4], [i32; 4], i32)> {
        layout(v)
            .iter()
            .filter(|q| f(q.tex))
            .map(|q| (q.tex, q.src.map(|s| s as i32), q.dst.map(|s| s as i32), q.alpha as i32))
            .collect()
    }

    /// Doubles, 5-5 → 5-6 in the first set of a best of three (as captured): the scorer's old 5 at 1.5× mid-rise,
    /// then the new 6 at 1.8× under its white copy (alpha 102) early in the drop.
    #[test]
    fn game_board() {
        let mut v = singles(1, -1, 3, false);
        v.players = 4;
        v.scorer = 1;
        v.sets_to_win = 2;
        v.set_games = [[5, 0, 0, 0, 0], [6, 0, 0, 0, 0]];
        v.show.event = Event::Game;
        let digits = |v: &View| quads(v, |t| t == Tex::Board(2)).into_iter().filter(|q| q.1[0] < 320).collect::<Vec<_>>();
        assert_eq!(
            digits(&v),
            [(Tex::Board(2), [160, 0, 32, 32], [272, 328, 32, 32], 128), (Tex::Board(2), [160, 0, 32, 32], [328, 320, 48, 48], 128)]
        );
        (v.show.stage, v.show.n) = (2, 8);
        assert_eq!(
            digits(&v),
            [
                (Tex::Board(2), [160, 0, 32, 32], [272, 328, 32, 32], 128),
                (Tex::Board(2), [192, 0, 32, 32], [323, 315, 57, 57], 128),
                (Tex::Board(2), [192, 32, 32, 32], [323, 315, 57, 57], 102),
            ]
        );
        // the frame: 16 px edges round a 304×80 middle from (152, 320)
        let frame = quads(&v, |t| t == Tex::Board(0));
        assert!(frame.contains(&(Tex::Board(0), [0, 0, 16, 16], [152, 320, 16, 16], 128)));
        assert!(frame.contains(&(Tex::Board(0), [40, 24, 16, 16], [472, 416, 16, 16], 128)));
        // later sets' columns are blank but for the dash; the sheets in draw order
        assert_eq!(quads(&v, |t| t == Tex::Board(1)).len(), 2);
        let order: Vec<_> = layout(&v).iter().map(|q| q.tex).collect();
        assert!(order.windows(2).all(|w| !matches!((w[0], w[1]), (Tex::Board(a), Tex::Board(b)) if a > b)));
        // the last set has its own finish
        v.show.event = Event::Set;
        v.match_over = true;
        assert!(layout(&v).is_empty());
    }

    /// Tiebreak 0-0 → 0-1 mid-swap: the old 0 slid up and fading, the new 1 under its fading white copy, the banner.
    #[test]
    fn tiebreak() {
        let mut v = singles(2, 2, 9, false);
        v.show.event = Event::TiebreakPoint;
        (v.points, v.scorer) = ([0, 1], 1);
        assert_eq!(
            quads(&v, |t| matches!(t, Tex::Tiebreak | Tex::TiebreakWhite | Tex::TiebreakBanner)),
            [
                (Tex::Tiebreak, [0, 0, 64, 64], [336, 284, 64, 64], 128),
                (Tex::Tiebreak, [0, 0, 64, 64], [336, 77, 64, 64], 85),
                (Tex::Tiebreak, [64, 0, 64, 64], [336, 104, 64, 64], 128),
                (Tex::TiebreakWhite, [64, 0, 64, 64], [336, 104, 64, 64], 43),
                (Tex::TiebreakBanner, [0, 0, 256, 64], [176, 32, 256, 64], 128),
            ]
        );
    }
}

// The finish banners and Set / Match Point.
//
// A point won outright with no line call can earn a banner: an untouched serve "Service Ace", an untouched return
// "Return Ace", a winning smash "Smash Ace", a winning counter "Counter" (the last of these wins), with "On the Line"
// under it when the deciding shot's first bounce mark touches or crosses a line. It flashes in white over 5 ticks,
// holds 45, fades over 5 (alpha 128·t/15 both ways, so it starts its fade at 42). Each new point that is a set or
// match point for a team, after one that wasn't, brings up "Set Point" / "Match Point" for 60 ticks, shrinking to
// 0.8 over its last 10, then growing to 1.5 as it fades over 10 more.

use hst_sim::judge::{Call, Rally};
use hst_sim::ps2::{add, div, mul, sub};
use hst_sim::score::{Rules, Score};

/// What the banners follow through the match (the original's shot record, scoreboard flags and bounce mark).
#[derive(Default)]
pub(super) struct Finish {
    /// Per player: the character's TParam Strk POW and Voley POW.
    pub power: Vec<[i32; 2]>,
    /// The shot before: hitter, branch (0 serve, 1 ground, 2 volley, 3 dive, 4 smash) and kind.
    prev: (usize, u8, i32),
    /// The last shot's branch and whether it was a counter.
    last: (u8, bool),
    /// The first bounce mark of the last shot's flight, its two ends (x, z), recorded while a point is on.
    mark: [[f32; 2]; 2],
    marking: bool,
    /// The finish banner on show: kind (0 service, 1 return, 2 smash ace, 3 counter), On the Line, stage, ticks.
    banner: Option<(usize, bool, u8, i32)>,
    /// Set (false) or Match (true) Point on show: stage and ticks; and whether the last new point wasn't one.
    point: Option<(bool, u8, i32)>,
    armed: bool,
}

/// A character's stroke and volley power from its TParam.csv row (the parser skips the empty Special POW cell).
pub(super) fn power(iso: &mut Iso, c: usize) -> [i32; 2] {
    let row = super::tparam(iso, c);
    [13, 14].map(|i| row[i].parse().expect("TParam power"))
}

impl Finish {
    /// A player strikes (`branch`, shot `kind`, `offset` frames off the sweet frame). A counter is a topspin or
    /// flat stroke on the sweet frame answering a topspin or flat ground stroke, volley or dive from a character
    /// with more power (Strk POW for a ground stroke, only with more than one player; Voley POW for a volley): it
    /// returns the power gap (the hit's own sound). `inside_high`: a topspin ground stroke from inside the service
    /// line on a ball at 0.6 or more, which the game plays as a volley and never as a counter.
    pub fn strike(&mut self, who: usize, branch: u8, kind: i32, offset: i32, players: usize, inside_high: bool) -> Option<i32> {
        let (prev, prev_branch, prev_kind) = std::mem::replace(&mut self.prev, (who, branch, kind));
        let flat_or_top = |k: i32| k == 0 || k == 2;
        let col = match branch {
            1 if players >= 2 && !inside_high => Some(0),
            2 => Some(1),
            _ => None,
        };
        let gap = col.filter(|_| {
            offset.abs() < 2 && flat_or_top(kind) && (1..=3).contains(&prev_branch) && flat_or_top(prev_kind)
        });
        let gap = gap.and_then(|c| {
            let (mine, theirs) = (self.power.get(who)?[c], self.power.get(prev)?[c]);
            (mine < theirs).then_some(theirs - mine)
        });
        self.last = (branch, gap.is_some());
        gap
    }

    /// The ball's first bounce off the court in a flight, at `at` moving at `vel` after it: the mark runs from the
    /// bounce along the ball's way by 0.8 of its level speed.
    pub fn bounce(&mut self, at: [f32; 3], vel: [f32; 3]) {
        if !self.marking {
            return;
        }
        let v = [sub(add(at[0], vel[0]), at[0]), sub(add(at[2], vel[2]), at[2])];
        let q = div(1.0, add(mul(v[0], v[0]), mul(v[1], v[1])).sqrt());
        let k = mul(add(mul(vel[0], vel[0]), mul(vel[2], vel[2])).sqrt(), 0.8);
        let a = [at[0], at[2]];
        self.mark = [a, [0, 1].map(|i| add(add(mul(mul(v[i], q), k), a[i]), 0.0))];
    }

    /// The point is decided (judge `call`, the rally's snapshot): a point won outright starts its banner.
    // ponytail: no body hits yet (an untouched ball is any ball); the start sound (slot 9) isn't loaded
    pub fn point_over(&mut self, rally: &Rally, call: u8, doubles: bool) {
        self.marking = false;
        if call != 0 {
            return;
        }
        let Some(kind) = finish_kind(rally.shots, rally.call, self.last) else { return };
        let on_line = on_the_line(self.mark, rally.shots < 2, doubles);
        self.banner = Some((kind, on_line, 1, 5));
    }

    /// A new point is set up: marks record afresh; a set or match point for either team after a point that wasn't
    /// one starts its banner.
    // ponytail: the jingle (slot 9) isn't loaded
    pub fn new_point(&mut self, score: &Score, rules: &Rules) {
        self.mark = [[0.0; 2]; 2];
        self.marking = true;
        match (0..2).find_map(|t| set_point(score, rules, t)) {
            None | Some(0) => self.armed = true,
            Some(r) => {
                if std::mem::take(&mut self.armed) {
                    self.point = Some((r > 1, 1, 60));
                }
            }
        }
    }

    /// One tick of both banners.
    pub fn tick(&mut self) {
        if let Some((_, _, stage, t)) = &mut self.banner {
            *t -= 1;
            if *t < 0 {
                match *stage {
                    1 => (*stage, *t) = (2, 45),
                    2 => (*stage, *t) = (3, 5),
                    _ => self.banner = None,
                }
            }
        }
        if let Some((_, stage, t)) = &mut self.point {
            *t -= 1;
            if *t < 0 {
                // ponytail: the game also restarts its flight sound object here (mode 1, 0x1b, 0x3c); not traced
                if *stage == 1 {
                    (*stage, *t) = (2, 10);
                } else {
                    self.point = None;
                }
            }
        }
    }
}

/// The finish banner a point won outright earns: 0 Service Ace (the serve), 1 Return Ace (the return), 2 Smash Ace
/// (a smash), 3 Counter; none after a net cord landing in. `last`: the last shot's branch and counter flag.
fn finish_kind(shots: i32, call: Call, last: (u8, bool)) -> Option<usize> {
    if call == Call::NetIn {
        return None;
    }
    match shots {
        1 => Some(0),
        2 => Some(1),
        3.. if last.1 => Some(3),
        3.. if last.0 == 4 => Some(2),
        _ => None,
    }
}

/// Whether a bounce mark from `a` to `b` (x, z) touches a line: an end within 0.1 of the nearest line across or
/// along, or the mark crossing the side line outwards; else it is on the line unless it starts beyond the end line
/// or ends short of it. A serve measures to the centre service line or the side line, and the service line.
fn on_the_line([a, b]: [[f32; 2]; 2], serve: bool, doubles: bool) -> bool {
    let d = |p: [f32; 2]| {
        let (x, z) = (p[0].abs(), p[1].abs());
        if serve {
            let side = sub(x, 4.115);
            (if x <= side.abs() { x } else { side }, sub(z, 6.4))
        } else {
            (sub(x, if doubles { 5.485 } else { 4.115 }), sub(z, 11.885))
        }
    };
    let ((ax, az), (bx, bz)) = (d(a), d(b));
    if [ax, bx, az, bz].iter().any(|v| v.abs() <= 0.1) {
        return true;
    }
    (ax <= 0.0 && bx >= 0.0) || !(az > 0.0 || bz < 0.0)
}

/// For team `t` at a game point: 0 just a game point, 1 a set point, 2 a match point; `None` without a game point.
fn set_point(s: &Score, r: &Rules, t: usize) -> Option<i32> {
    if !s.game_point(r, t) {
        return None;
    }
    let lead = (s.games[t] - s.games[t ^ 1] > 0) as i32;
    if !s.tiebreak && s.games[t] < r.games - lead {
        return Some(0);
    }
    // ponytail: the game's match point 3 (only the other team on a controller) differs only in the jingle's pitch
    Some(if s.sets[t] < r.sets - 1 { 1 } else { 2 })
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BannerTex {
    /// `inpane_finish`: each finish's word in white, 64 rows apart from 24, "On the Line" at 232.
    White,
    /// `inpane_finish00`–`02`, `inpane_Counter`: the words from row 24.
    Finish(usize),
    /// `inpane_finish03`: "On the Line".
    OnLine,
    Match,
    Set,
}

const BANNER_ART: [&str; 8] = [
    "/inpane_finish.tm2",
    "/inpane_finish00.tm2",
    "/inpane_finish01.tm2",
    "/inpane_finish02.tm2",
    "/inpane_counter.tm2",
    "/inpane_finish03.tm2",
    "/inpane_match.tm2",
    "/inpane_set.tm2",
];

/// The banners' quads, in the original's draw order (the finish word, On the Line, the white flash; then Set /
/// Match Point).
fn banners(f: &Finish) -> Vec<(BannerTex, [f32; 4], [f32; 4], i32)> {
    let mut out = Vec::new();
    if let Some((kind, on_line, stage, t)) = f.banner {
        let (x, y) = (208.0, if on_line { 192.0 } else { 212.0 });
        let a = if stage == 3 { (t << 7) / 15 } else { 128 };
        out.push((BannerTex::Finish(kind), [0.0, 24.0, 224.0, 40.0], [x, y, 224.0, 40.0], a));
        let under = [x + 24.0, y + 40.0, 224.0, 24.0];
        if on_line {
            out.push((BannerTex::OnLine, [0.0, 0.0, 224.0, 24.0], under, a));
        }
        if stage == 1 {
            let flash = (t << 7) / 15;
            out.push((BannerTex::White, [0.0, (kind * 64 + 24) as f32, 224.0, 40.0], [x, y, 224.0, 40.0], flash));
            if on_line {
                out.push((BannerTex::White, [0.0, 232.0, 224.0, 24.0], under, flash));
            }
        }
    }
    if let Some((is_match, stage, t)) = f.point {
        let (s, a) = if stage == 1 {
            (if t < 11 { (1.0 - 0.02 * (10 - t) as f64) as f32 } else { 1.0 }, 128)
        } else {
            (0.8 + (0.07 * (10 - t) as f64) as f32, (t << 7) / 10)
        };
        let (w, h) = (256.0, 48.0);
        let dst = [192.0 + w / 2.0 - w * s / 2.0, 176.0 + h / 2.0 - h * s / 2.0, w * s, h * s];
        out.push((if is_match { BannerTex::Match } else { BannerTex::Set }, [0.0, 0.0, w, h], dst, a));
    }
    out.retain(|q| q.3 > 0);
    out
}

#[derive(Resource)]
struct BannerArt([Handle<Image>; 8]);
#[derive(Component)]
struct BannerSlot(usize);

fn setup_banners(mut commands: Commands, args: Res<Args>, mut images: ResMut<Assets<Image>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso.read("AZUMA/INPANE/INPANE.XB0").expect("INPANE archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let art = BANNER_ART.map(|name| {
        let e = arc
            .entries
            .iter()
            .find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name))
            .unwrap_or_else(|| panic!("{name} in INPANE"));
        panel::image(&mut images, &arc.read(e).expect("INPANE bytes"))
    });
    commands.insert_resource(BannerArt(art));
    commands
        .spawn((Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, GlobalZIndex(1)))
        .with_children(|p| {
            for i in 0..5 {
                p.spawn((BannerSlot(i), ImageNode { image_mode: NodeImageMode::Stretch, ..default() }, Node { position_type: PositionType::Absolute, ..default() }, Visibility::Hidden));
            }
        });
}

fn tick_banners(mut g: ResMut<Game>) {
    g.finish.tick();
}

fn draw_banners(g: Res<Game>, art: Option<Res<BannerArt>>, mut q: Query<(&BannerSlot, &mut ImageNode, &mut Node, &mut Visibility)>) {
    let Some(art) = art else { return };
    let quads = banners(&g.finish);
    for (BannerSlot(i), mut img, mut node, mut vis) in &mut q {
        let Some(&(tex, [u, v, w, h], [x, y, dw, dh], alpha)) = quads.get(*i) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let k = match tex {
            BannerTex::White => 0,
            BannerTex::Finish(k) => 1 + k,
            BannerTex::OnLine => 5,
            BannerTex::Match => 6,
            BannerTex::Set => 7,
        };
        img.image = art.0[k].clone();
        img.rect = Some(Rect::new(u, v, u + w, v + h));
        img.color = Color::srgba(1.0, 1.0, 1.0, alpha as f32 / 128.0);
        node.left = Val::Percent(x / 6.4);
        node.top = Val::Percent(y / 4.48);
        node.width = Val::Percent(dw / 6.4);
        node.height = Val::Percent(dh / 4.48);
        *vis = Visibility::Inherited;
    }
}

#[cfg(test)]
mod banner_tests {
    use super::*;

    #[test]
    fn kinds() {
        assert_eq!(finish_kind(1, Call::In, (0, false)), Some(0));
        assert_eq!(finish_kind(1, Call::NetIn, (0, false)), None);
        assert_eq!(finish_kind(2, Call::In, (1, false)), Some(1));
        assert_eq!(finish_kind(5, Call::In, (4, false)), Some(2));
        assert_eq!(finish_kind(5, Call::NetIn, (4, false)), None);
        assert_eq!(finish_kind(4, Call::In, (2, true)), Some(3));
        assert_eq!(finish_kind(4, Call::In, (1, false)), None);
    }

    #[test]
    fn counter() {
        let mut f = Finish { power: vec![[4, 3], [10, 16]], ..default() };
        // a flat ground stroke from player 1, answered by player 0's volley on the sweet frame
        assert_eq!(f.strike(1, 1, 2, 0, 2, false), None);
        assert_eq!(f.strike(0, 2, 0, 1, 2, false), Some(13));
        assert_eq!(f.last, (2, true));
        // the stronger player's answer, a slice, or off the sweet frame are no counters
        assert_eq!(f.strike(1, 2, 0, 0, 2, false), None);
        assert_eq!(f.strike(0, 1, 1, 0, 2, false), None);
        f.prev = (1, 1, 0);
        assert_eq!(f.strike(0, 1, 0, 2, 2, false), None);
        f.prev = (1, 1, 0);
        assert_eq!(f.strike(0, 1, 0, -1, 2, true), None);
        f.prev = (1, 1, 0);
        assert_eq!(f.strike(0, 1, 0, -1, 2, false), Some(6));
        // nothing answers a serve or a smash
        f.prev = (1, 0, 0);
        assert_eq!(f.strike(0, 2, 0, 0, 2, false), None);
    }

    #[test]
    fn line() {
        // a rally ball skidding over the singles side line, one well inside, one past the base line
        assert!(on_the_line([[4.0, 8.0], [4.3, 9.0]], false, false));
        assert!(!on_the_line([[3.0, 8.0], [3.5, 9.0]], false, false));
        assert!(!on_the_line([[3.0, 12.0], [3.5, 13.0]], false, false));
        assert!(on_the_line([[3.0, 11.5], [3.2, 12.5]], false, false));
        // doubles alley; a serve near the centre service line
        assert!(!on_the_line([[4.0, 8.0], [4.3, 9.0]], false, true));
        assert!(on_the_line([[0.05, 5.0], [0.5, 5.5]], true, false));
    }

    #[test]
    fn set_points() {
        let r = Rules { sets: 2, games: 6, no_deuce: false, one_point_games: false, players: 2 };
        let mut s = Score::new();
        (s.points, s.games) = ([3, 0], [5, 4]);
        assert_eq!(set_point(&s, &r, 0), Some(1));
        assert_eq!(set_point(&s, &r, 1), None);
        s.games = [5, 5];
        assert_eq!(set_point(&s, &r, 0), Some(0));
        (s.games, s.sets) = ([5, 3], [1, 0]);
        assert_eq!(set_point(&s, &r, 0), Some(2));
        // shown once, on the first set point after one that wasn't
        let mut f = Finish::default();
        f.new_point(&Score::new(), &r);
        f.new_point(&s, &r);
        assert_eq!(f.point, Some((true, 1, 60)));
        f.point = None;
        f.new_point(&s, &r);
        assert_eq!(f.point, None);
    }

    #[test]
    fn timeline() {
        let mut f = Finish { banner: Some((0, true, 1, 5)), point: Some((false, 1, 60)), ..default() };
        let mut seen = Vec::new();
        for _ in 0..80 {
            f.tick();
            let q = banners(&f);
            let word = q.iter().find(|q| q.0 == BannerTex::Finish(0)).map(|q| q.3);
            let flash = q.iter().find(|q| q.0 == BannerTex::White).map(|q| q.3);
            let set = q.iter().find(|q| q.0 == BannerTex::Set).map(|q| (q.2[0], q.2[2], q.3));
            seen.push((word, flash, set));
        }
        // flash 34 → 8 over the first ticks, then the 45-tick hold, then the fade from 42
        assert_eq!(seen[0].0, Some(128));
        assert_eq!(seen[0].1, Some(34));
        assert_eq!(seen[3].1, Some(8));
        assert_eq!(seen[4].1, None);
        assert_eq!(seen[50].0, Some(128));
        assert_eq!(seen[51].0, Some(42));
        assert_eq!(seen[56].0, None);
        // Set Point: full size until 10 ticks remain, 0.8 at the end of the hold, then growing as it fades
        assert_eq!(seen[48].2, Some((192.0, 256.0, 128)));
        assert_eq!(seen[59].2.map(|s| s.2), Some(128));
        assert_eq!(seen[60].2, Some((217.6, 204.8, 128)));
        assert_eq!(seen[69].2.map(|s| s.2), Some(12));
        assert_eq!(seen[70].2, None);
    }
}
