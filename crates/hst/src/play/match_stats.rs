//! The match statistics (`hst_sim::stats`), counted over the match, and the original's stats screen at its end
//! (textures `result_status00`/`01` from `AZUMA/PRIZE/OTHER.XB0`, coordinates on the 640×448 screen): a column
//! per player under a header pill (face, slot label) and a column of row names, the 9 rows' values (the best of
//! each row on the player's pill colour, but net play and forehand rates), the longest rally and the match time
//! on the bar below. The winner's header pulses white.
//!
//! The match pauses on it; ✕ starts the next match. `HST_STATS=<seconds>` ends the match (as it stands) after that
//! long, for `--shot`.
//! ponytail: the original's match-over screen (the result page with its banner, "Stats Screen" beside
//! "✕ Continue", the page slide) isn't ported; the stats page opens alone. The row names' top-to-bottom tint is one
//! colour, the background behind the columns (a stretched sprite of the result screen's base) is plain white and
//! the characters' names under the faces are left out.

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::score::{Event, Score};
use hst_sim::serve::{self, Balloon};
use hst_sim::stats::{Table, hms};
use std::time::Duration;

use super::panel::{self, Colours};
use super::{Game, Pads, Phase};
use crate::Args;

const TEXTURES: [&str; 2] = ["/result_status00.tm2", "/result_status01.tm2"];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tex {
    Frame,
    Text,
    /// A plain white quad.
    Fill,
    Face(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Quad {
    tex: Tex,
    src: [f32; 4],
    dst: [f32; 4],
    rgb: [f32; 3],
    alpha: f32,
}

const WHITE: [f32; 3] = [128.0; 3];
/// ponytail: the row names' vertex colours run (111,114,58) at the top to (127,127,127) at the bottom; one colour
const LABEL: [f32; 3] = [119.0, 120.0, 92.0];

/// The struck shot's counts: shot number, contact branch, the forehand side, the sweet balloon, the net distance.
pub(super) fn hit(g: &mut Game, who: usize, branch: u8, grade: u8, offset: i32) {
    let sweet = serve::balloon(grade, offset, branch == 3) == Some(Balloon::Sweet);
    let (forehand, z) = (!g.players[who].backhand, g.players[who].pos[2]);
    g.match_stats.hit(who, g.shots, g.score.server as usize, branch, forehand, sweet, z);
}

pub(super) fn point(g: &mut Game, call: u8, winner: Option<usize>) {
    let server = g.score.server as usize;
    g.match_stats.point(call, winner, g.shots, server, g.rally.faults);
}

/// A game (or tiebreak) won by `team`: its final points.
pub(super) fn scored(g: &mut Game, before: &Score, event: Option<Event>, team: usize) {
    if matches!(event, Some(Event::Game | Event::Set)) {
        let mut points = before.points;
        points[team] += 1;
        g.match_stats.game(points);
    }
}

pub(super) fn match_over(g: &mut Game) {
    let table = g.match_stats.finish(g.post_winner.clamp(0, 1) as usize);
    g.match_stats = hst_sim::stats::Stats::new();
    g.match_stats.result = Some(table);
}

/// What the stats screen shows.
struct View<'a> {
    table: &'a Table,
    players: usize,
    slots: [usize; 4],
    pill: [[u8; 3]; 4],
    /// The winner's header alpha (pulsing).
    pulse: i32,
}

fn layout(v: &View) -> Vec<Quad> {
    let mut out = Vec::new();
    let mut q = |tex, src: [i32; 4], dst: [i32; 4], rgb: [f32; 3], alpha: i32| {
        out.push(Quad { tex, src: src.map(|c| c as f32), dst: dst.map(|c| c as f32), rgb, alpha: alpha as f32 })
    };
    let (n, t) = (v.players, v.table);
    let singles = n < 3;
    let rgb = |c: [u8; 3]| c.map(f32::from);
    // native size where the original passes −1
    let nat = |src: [i32; 4], x: i32, y: i32| [x, y, src[2], src[3]];
    let digit = |d: u32| [d as i32 * 16, 0, 16, 24];

    let (x0, w) = if singles { (136, 368) } else { (40, 560) };
    q(Tex::Fill, [0, 0, 8, 8], [x0, 80, w, 256], WHITE, 128);
    // the bar below: longest rally, match time
    q(Tex::Frame, [0, 472, 16, 40], [40, 352, 16, 40], WHITE, 128);
    q(Tex::Frame, [16, 472, 16, 40], [56, 352, 528, 40], WHITE, 128);
    q(Tex::Frame, [32, 472, 16, 40], [584, 352, 16, 40], WHITE, 128);
    let rally = t.longest_rally.min(999);
    q(Tex::Text, [0, 320, 104, 24], [88, 359, 104, 24], WHITE, 128);
    for (k, d) in [rally / 100, rally / 10 % 10, rally % 10].into_iter().enumerate() {
        if (k == 0 && rally < 100) || (k == 1 && rally < 10) {
            continue;
        }
        q(Tex::Text, digit(d), nat(digit(d), 216 + 16 * k as i32, 359), WHITE, 128);
    }
    q(Tex::Text, [0, 344, 80, 24], [352, 359, 80, 24], WHITE, 128);
    for (k, part) in hms(t.frames).into_iter().enumerate() {
        let x = 440 + 40 * k as i32;
        q(Tex::Text, digit(part / 10 % 10), [x, 359, 16, 24], WHITE, 128);
        q(Tex::Text, digit(part % 10), [x + 16, 359, 16, 24], WHITE, 128);
        if k < 2 {
            q(Tex::Text, [80, 344, 8, 24], [x + 32, 359, 8, 24], WHITE, 128);
        }
    }
    // each player's header pill, in the player's stripe
    for p in 0..n {
        let vv = 0x110 + 0x28 * p as i32;
        let x = [x0, 408][p & 1] + if p >= 2 { 96 } else { 0 };
        if p & 1 == 0 {
            q(Tex::Frame, [0, vv, 8, 40], [x, 80, 24, 40], WHITE, 128);
            q(Tex::Frame, [8, vv, 16, 40], [x + 24, 80, 16, 40], WHITE, 128);
            q(Tex::Frame, [24, vv, 7, 40], [x + 40, 80, 56, 40], WHITE, 128);
        } else {
            q(Tex::Frame, [0, vv, 8, 40], [x, 80, 56, 40], WHITE, 128);
            q(Tex::Frame, [8, vv, 16, 40], [x + 56, 80, 16, 40], WHITE, 128);
            q(Tex::Frame, [24, vv, 7, 40], [x + 72, 80, 24, 40], WHITE, 128);
        }
    }
    let wx = match (t.winner, singles) {
        (0, true) => 0x88,
        (0, false) => 0x28,
        _ => 0x198,
    };
    q(Tex::Frame, [44, 308, 4, 4], [wx, 80, if singles { 0x60 } else { 0xc0 }, 40], WHITE, v.pulse);
    // faces and slot labels: even players face then label, odd players label then face
    let (faces, labels): (&[i32], &[i32]) =
        if singles { (&[0x8a, 0x1d0], &[0xc6, 0x19a]) } else { (&[0x28, 0x1d0, 0x8a, 0x230], &[0x66, 0x19a, 0xc6, 0x1fa]) };
    for p in 0..n {
        q(Tex::Face(p), [0, 0, 64, 64], [faces[p], 80, 64, 64], WHITE, 128);
        let lx = labels[p] - if p & 1 == 0 { 14 } else { 0 };
        let src = [0, v.slots[p] as i32 * 24 + 392, 48, 24];
        q(Tex::Text, src, nat(src, lx, 80), rgb(v.pill[p]), 128);
    }
    // each row's best (fewest missed shots and double faults, the most of the rest) on its players' colours
    let hx: &[i32] = if singles { &[0x88, 0x198] } else { &[0x28, 0x198, 0x88, 0x1f8] };
    for r in (0..9).filter(|r| !(6..=7).contains(r)) {
        let who = if r == 8 { n.min(2) } else { n };
        let vals: Vec<f32> = (0..who).map(|p| t.rows[p][r]).collect();
        let best = if (2..=3).contains(&r) {
            vals.iter().copied().fold(f32::MAX, f32::min)
        } else {
            vals.iter().copied().fold(f32::MIN, f32::max)
        };
        let ties = (0..n).filter(|&p| t.rows[p][r] == best).count();
        if ties == n {
            continue;
        }
        for p in (0..who).filter(|&p| vals[p] == best) {
            let w = if r == 8 && !singles { 192 } else { 96 };
            q(Tex::Frame, [49, 273, 6, 8], [hx[p], 120 + 24 * r as i32, w, 24], rgb(v.pill[p]), 128);
        }
    }
    // the column frames and their dividers
    let fx = if singles { 128 } else { 32 };
    q(Tex::Frame, [0, 0, 16, 272], [fx, 72, 16, 272], WHITE, 128);
    q(Tex::Frame, [16, 0, 8, 272], [fx + 16, 72, 88, 272], WHITE, 128);
    if !singles {
        q(Tex::Frame, [16, 0, 8, 272], [fx + 104, 72, 96, 272], WHITE, 128);
    }
    let i = fx + if singles { 104 } else { 200 };
    q(Tex::Frame, [29, 0, 6, 272], [i, 72, 176, 272], WHITE, 128);
    q(Tex::Frame, [16, 0, 8, 272], [i + 176, 72, 88, 272], WHITE, 128);
    if !singles {
        q(Tex::Frame, [16, 0, 8, 272], [i + 264, 72, 96, 272], WHITE, 128);
    }
    q(Tex::Frame, [40, 0, 16, 272], [i + if singles { 264 } else { 360 }, 72, 16, 272], WHITE, 128);
    for (k, x) in [136, 232, 406, 504].into_iter().enumerate() {
        if singles && (k == 0 || k == 3) {
            continue;
        }
        let h = if k == 0 || k == 3 { 232 } else { 256 };
        q(Tex::Frame, [40, 273, 2, 14], [x, 80, 2, h], WHITE, 128);
    }
    // "Stats" and the row names
    q(Tex::Text, [0, 64, 160, 40], [240, 80, 160, 40], LABEL, 128);
    for k in 0..9 {
        q(Tex::Text, [0, 104 + 24 * k, 160, 24], [240, 120 + 24 * k, 160, 24], LABEL, 128);
    }
    // the values
    let (a, b, d, e): (&[i32], &[i32], &[i32], &[i32]) = if singles {
        (&[0xd2, 0x1e2], &[0x8a, 0x19a], &[0x92, 0x1a2], &[0x92, 0x1a2])
    } else {
        (&[0x70, 0x1e0, 0xd2, 0x242], &[0x28, 0x198, 0x8a, 0x1fa], &[0x30, 0x1a0, 0x92, 0x202], &[0x58, 0x1c8])
    };
    // `digits` digits right-aligned at x, leading zeros hidden
    let number = |out: &mut Vec<Quad>, value: u32, digits: u32, x: i32, y: i32| {
        for k in 0..digits {
            let place = 10u32.pow(digits - 1 - k);
            if k + 1 < digits && value < place {
                continue;
            }
            let src = digit(value / place % 10);
            out.push(Quad {
                tex: Tex::Text,
                src: src.map(|c| c as f32),
                dst: [(x + 16 * k as i32) as f32, y as f32, 16.0, 24.0],
                rgb: WHITE,
                alpha: 128.0,
            });
        }
    };
    let mut values = Vec::new();
    for r in 0..9 {
        let y = 120 + 24 * r as i32;
        for p in 0..n {
            let v = t.rows[p][r];
            match r {
                0 => {
                    values.push(text([0, 24, 48, 24], b[p] + 48, y));
                    number(&mut values, v as u32, 3, b[p], y);
                }
                1..=4 => number(&mut values, v as u32, 4, d[p], y),
                5..=7 => {
                    values.push(text([72, 24, 24, 24], a[p], y));
                    percent(&mut values, v, b[p], y);
                }
                _ if p < 2 => {
                    let at = if singles { a[p] } else { [152, 520][p] };
                    values.push(text([96, 24, 24, 24], at, y));
                    number(&mut values, v as u32, 3, e[p], y);
                }
                _ => {}
            }
        }
    }
    out.extend(values);
    out
}

fn text(src: [i32; 4], x: i32, y: i32) -> Quad {
    Quad {
        tex: Tex::Text,
        src: src.map(|c| c as f32),
        dst: [x as f32, y as f32, src[2] as f32, src[3] as f32],
        rgb: WHITE,
        alpha: 128.0,
    }
}

/// A rate with one decimal: hundreds only from 100, tens only from 10, the point after the units.
fn percent(out: &mut Vec<Quad>, v: f32, x: i32, y: i32) {
    let tenths = (v * 10.0) as u32;
    let whole = tenths / 10;
    let cell = |d: u32, x: i32| text([d as i32 * 16, 0, 16, 24], x, y);
    if whole >= 100 {
        out.push(cell(whole / 100 % 10, x));
    }
    if whole >= 10 {
        out.push(cell(whole / 10 % 10, x + 16));
    }
    out.push(cell(whole % 10, x + 32));
    out.push(text([160, 0, 8, 24], x + 48, y));
    out.push(cell(tenths % 10, x + 56));
}

/// The winner's header pulse: 0 up to 102 and back over 21 ticks each way.
#[derive(Default)]
struct Pulse {
    down: bool,
    c: i32,
}

impl Pulse {
    fn step(&mut self) -> i32 {
        let a = if self.down { self.c * 0x66 / 0x14 } else { 0x66 - self.c * 0x66 / 0x14 };
        self.c -= 1;
        if self.c < 0 {
            (self.c, self.down) = (0x14, !self.down);
        }
        a
    }
}

#[derive(Resource)]
struct Art(Vec<Handle<Image>>);
#[derive(Component)]
struct Slot(usize);
#[derive(Component)]
struct Root;
const POOL: usize = 200;

#[derive(Resource, Default)]
struct Screen {
    pulse: Pulse,
    alpha: i32,
    acc: f32,
    open: bool,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Screen>()
        .add_systems(PostStartup, setup.after(super::setup))
        .add_systems(FixedUpdate, tick.after(super::simulate))
        .add_systems(Update, (step, draw).chain());
}

fn setup(mut commands: Commands, args: Res<Args>, mut images: ResMut<Assets<Image>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso.read("AZUMA/PRIZE/OTHER.XB0").expect("PRIZE archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let art = TEXTURES
        .iter()
        .map(|name| {
            let e = arc
                .entries
                .iter()
                .find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name))
                .unwrap_or_else(|| panic!("{name} in PRIZE/OTHER"));
            panel::image(&mut images, &arc.read(e).expect("PRIZE bytes"))
        })
        .collect();
    commands.insert_resource(Art(art));
    commands
        .spawn((super::widescreen::screen_43(), GlobalZIndex(11), Root))
        .with_children(|p| {
            for i in 0..POOL {
                p.spawn((
                    Slot(i),
                    ImageNode { image_mode: NodeImageMode::Stretch, ..default() },
                    Node { position_type: PositionType::Absolute, ..default() },
                    Visibility::Hidden,
                ));
            }
        });
}

/// A frame of the match: its time, a new serve, the serve's speed the frame after it is struck.
fn tick(mut g: ResMut<Game>, mut serving: Local<bool>) {
    let g = &mut *g;
    g.match_stats.tick();
    let serve = g.phase == Phase::Serve;
    if serve && !*serving {
        g.match_stats.serve();
    }
    *serving = serve;
    if g.shots == 1 && g.phase == Phase::Rally {
        g.match_stats.serve_speed(hst_sim::sound::kmh(g.flight.ball.vel));
    }
}

/// Opens the screen when a match has ended (holding the match's fixed clock), steps its pulse, ✕ closes it.
fn step(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    mut fixed: ResMut<Time<Fixed>>,
    mut g: ResMut<Game>,
    mut screen: ResMut<Screen>,
    mut pads: ResMut<Pads>,
    mut auto: Local<bool>,
) {
    if !*auto
        && std::env::var("HST_STATS").ok().and_then(|s| s.parse::<f32>().ok()).is_some_and(|t| time.elapsed_secs() >= t)
    {
        *auto = true;
        let g = &mut *g;
        let winner = (g.score.games[1] > g.score.games[0]) as i32;
        g.post_winner = winner;
        match_over(g);
    }
    if g.match_stats.result.is_none() {
        return;
    }
    if !screen.open {
        info!("stats screen opens");
        *screen = Screen { open: true, pulse: Pulse { down: false, c: 0x14 }, ..default() };
        // ponytail: the fixed clock runs on but never reaches a step, as the pause menu
        fixed.set_timestep(Duration::from_secs(1 << 20));
    }
    screen.acc += time.delta_secs() * 60.0;
    while screen.acc >= 1.0 {
        screen.acc -= 1.0;
        screen.alpha = screen.pulse.step();
    }
    let confirm = keys.any_just_pressed([KeyCode::Enter, KeyCode::Space, KeyCode::KeyJ])
        || gamepads.iter().any(|p| p.just_pressed(GamepadButton::South));
    for s in &mut pads.slots {
        s.shot = None;
        s.serve = false;
    }
    if confirm {
        g.match_stats.result = None;
        screen.open = false;
        fixed.set_timestep_hz(60.0);
        let over = fixed.overstep();
        fixed.discard_overstep(over);
    }
}

fn draw(
    g: Res<Game>,
    pads: Res<Pads>,
    screen: Res<Screen>,
    art: Option<Res<Art>>,
    panel_art: Option<Res<panel::Art>>,
    colours: Option<Res<Colours>>,
    mut q: Query<(&Slot, &mut ImageNode, &mut Node, &mut Visibility)>,
) {
    let (Some(art), Some(panel_art), Some(colours)) = (art, panel_art, colours) else {
        return;
    };
    let n = g.players.len();
    let quads = match &g.match_stats.result {
        Some(table) if screen.open => layout(&View {
            table,
            players: n,
            slots: std::array::from_fn(|i| if i < n { pads.slot_of(i, n).unwrap_or(4) } else { 4 }),
            pill: colours.0,
            pulse: screen.alpha,
        }),
        _ => Vec::new(),
    };
    for (Slot(i), mut img, mut node, mut vis) in &mut q {
        let Some(quad) = quads.get(*i) else {
            *vis = Visibility::Hidden;
            continue;
        };
        img.image = match quad.tex {
            Tex::Frame => art.0[0].clone(),
            Tex::Text => art.0[1].clone(),
            Tex::Fill => Handle::default(),
            Tex::Face(p) => panel_art.0[panel::FACES + p].clone(),
        };
        let [u, v, w, h] = quad.src;
        img.rect = (quad.tex != Tex::Fill).then(|| Rect::new(u, v, u + w, v + h));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Table {
        let mut rows = [[0.0; 9]; 4];
        rows[0] = [161.0 * 0.62137, 24.0, 3.0, 1.0, 2.0, 37.5, 0.0, 60.0, 64.0];
        rows[1] = [149.0 * 0.62137, 18.0, 7.0, 1.0, 0.0, 12.25, 10.0, 55.0, 36.0];
        Table { rows, longest_rally: 7, frames: 60 * 754, winner: 0 }
    }

    /// Singles: the values' digits where the original draws them, the bests highlighted, ties not.
    #[test]
    fn singles() {
        let t = table();
        let v = View { table: &t, players: 2, slots: [0, 4, 4, 4], pill: [[98, 46, 61], [33, 74, 97], [0; 3], [0; 3]], pulse: 51 };
        let quads = layout(&v);
        let at = |x: f32, y: f32| quads.iter().filter(|q| q.dst[0] == x && q.dst[1] == y).map(|q| q.src).collect::<Vec<_>>();
        // fastest serve: 161 km/h is 100 mph at 0x8a
        assert_eq!(at(138.0, 120.0), vec![[16.0, 0.0, 16.0, 24.0]]);
        assert_eq!(at(170.0, 120.0), vec![[0.0, 0.0, 16.0, 24.0]]);
        // points won 24 right-aligned in 4 digits from 0x92: "2" at +32, "4" at +48
        assert_eq!(at(0x92 as f32 + 32.0, 144.0), vec![[32.0, 0.0, 16.0, 24.0]]);
        assert!(at(0x92 as f32, 144.0).is_empty());
        // 12.25 %: "1" "2" "." "2" from 0x19a (no hundreds)
        assert_eq!(at(0x19a as f32 + 16.0, 240.0), vec![[16.0, 0.0, 16.0, 24.0]]);
        assert_eq!(at(0x19a as f32 + 48.0, 240.0), vec![[160.0, 0.0, 8.0, 24.0]]);
        assert_eq!(at(0x19a as f32 + 56.0, 240.0), vec![[32.0, 0.0, 16.0, 24.0]]);
        // highlights: player 0 for points (row 1), missed (2); double faults tie (3); never net play (6)
        let lit = |r: i32| quads.iter().filter(|q| q.src == [49.0, 273.0, 6.0, 8.0] && q.dst[1] == (120 + 24 * r) as f32).map(|q| q.dst[0]).collect::<Vec<_>>();
        assert_eq!(lit(1), vec![136.0]);
        assert_eq!(lit(2), vec![136.0]);
        assert!(lit(3).is_empty());
        assert!(lit(6).is_empty());
        // 12:34 match time: 00:12:34
        assert_eq!(at(480.0 + 16.0, 359.0), vec![[32.0, 0.0, 16.0, 24.0]]);
        assert_eq!(at(520.0, 359.0), vec![[48.0, 0.0, 16.0, 24.0]]);
        assert!(quads.len() <= POOL);
    }

    #[test]
    fn doubles_fits() {
        let t = table();
        let v = View { table: &t, players: 4, slots: [0, 4, 4, 4], pill: [[128; 3]; 4], pulse: 0 };
        assert!(layout(&v).len() <= POOL);
    }

    #[test]
    fn pulse() {
        let mut p = Pulse { down: false, c: 0x14 };
        let a: Vec<i32> = (0..44).map(|_| p.step()).collect();
        assert_eq!((a[0], a[20], a[21], a[41], a[42]), (0, 102, 102, 0, 0));
    }
}
