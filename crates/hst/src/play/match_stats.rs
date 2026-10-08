//! The match statistics (`hst_sim::stats`), counted over the match, and the original's stats screen at its end
//! (textures `result_status00`/`01` from `AZUMA/PRIZE/OTHER.XB0`, coordinates on the 640×448 screen): a column
//! per player under a header pill (face, slot label) and a column of row names, the 9 rows' values (the best of
//! each row on the player's pill colour, but net play and forehand rates), the longest rally and the match time
//! on the bar below. The winner's header pulses white.
//!
//! Behind the columns a dark sprite (`i_pause_result_15`) at half alpha; "Stats" fades top to bottom from olive to
//! grey, the row names are plain; each player's rank (`inpane_dani`, in the rank colour) under the slot label.
//!
//! It is the second page of the original's result screen. The first is the match's final board (the result board
//! with every set settled, 40 up, the winner's stripes under the pulsing highlight and its sets count pulsing in
//! scale) under the "Game, Set, Match!" model (`i_gameset_00`, `AZUMA/PRIZE/OTHER.XB0`). → / ← slide 40 px a tick
//! to the other page (✕ too, while the stats page hasn't been seen); along the bottom (`KeyAssign_inpane`, the
//! menus' `info` icons) the other page's name beside the blinking d-pad, and from tick 120 "✕ Continue", which
//! ends it once the stats page has been seen.
//!
//! The match pauses on it; Continue starts the next match. `HST_STATS=<seconds>` ends the match (as it stands) after
//! that long, for `--shot`; `HST_STATS_PAGE=1` opens it on the stats page.
//! ponytail: the original's first state (the winners' ceremony with "NARROW WIN" etc. and the confetti) isn't
//! ported (P26c): the result page opens straight away, the banner's animation run on by that state's length.

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::score::{Event, Score};
use hst_sim::serve::{self, Balloon};
use hst_sim::stats::{Table, hms};
use std::time::Duration;

use super::panel::{self, Colours};
use super::popups::{self, Final};
use super::{Game, Pads, Phase};
use crate::Args;

/// From `AZUMA/PRIZE/OTHER.XB0`, then `AZUMA/INPANE/INPANE.XB0`.
const TEXTURES: [&str; 3] = ["/result_status00.tm2", "/result_status01.tm2", "/keyassign_inpane.tm2"];
const INPANE: [&str; 2] = ["/i_pause_result_15.tm2", "/2d/info.tm2"];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tex {
    Frame,
    Text,
    /// "Score Screen", "Stats Screen", "Continue", 152×24 rows.
    Keys,
    /// The dark sprite behind the columns.
    Back,
    /// The menus' button icons.
    Info,
    Face(usize),
    Rank,
    /// The result board's (popups) sheets.
    Board(u8),
    Slot,
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
/// "Stats"'s vertex colours, top and bottom.
const FADE: [[f32; 3]; 2] = [[111.0, 114.0, 58.0], [127.0, 127.0, 127.0]];

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
    ranks: [Option<usize>; 4],
    rank_rgb: [[u8; 3]; 4],
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
    q(Tex::Back, [0, 0, 8, 8], [x0, 80, w, 256], WHITE, 64);
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
        // the rank under it: beside the face on the left half
        if let Some(r) = v.ranks[p] {
            let x = if lx <= 320 { faces[p] + 61 } else { lx } - if p & 1 == 0 { 4 } else { 0 };
            q(Tex::Rank, [0, r as i32 * 24, 64, 24], [x, 104, 64, 24], rgb(v.rank_rgb[p]), 128);
        }
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
    // "Stats", its vertex colours run down a scanline at a time, and the row names
    for r in 0..40 {
        let t = (r as f32 + 0.5) / 40.0;
        let c = std::array::from_fn(|i| FADE[0][i] + (FADE[1][i] - FADE[0][i]) * t);
        q(Tex::Text, [0, 64 + r, 160, 1], [240, 80 + r, 160, 1], c, 128);
    }
    for k in 0..9 {
        q(Tex::Text, [0, 104 + 24 * k, 160, 24], [240, 120 + 24 * k, 160, 24], WHITE, 128);
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

/// A sprite at its native size.
fn sprite(tex: Tex, src: [i32; 4], x: i32, y: i32) -> Quad {
    Quad { tex, src: src.map(|c| c as f32), dst: [x as f32, y as f32, src[2] as f32, src[3] as f32], rgb: WHITE, alpha: 128.0 }
}

/// The bottom bar: from tick 120 "✕ Continue" (the ✕ blinking in), always the other page's name beside the d-pad
/// (blinking to the side it lies).
fn bar(page: usize, timer: i32, blink: i32) -> Vec<Quad> {
    let mut out = Vec::new();
    if timer >= 120 {
        out.push(sprite(Tex::Keys, [0, 48, 152, 24], 288, 404));
        if blink > 0x13 {
            out.push(sprite(Tex::Info, [144, 160, 32, 32], 256, 400));
        }
        out.push(sprite(Tex::Info, [224, 96, 32, 32], 256, 400));
    }
    out.push(sprite(Tex::Keys, [0, (page == 0) as i32 * 24, 152, 24], 472, 404));
    let pad = if blink < 0x14 { [192, if page == 0 { 64 } else { 32 }, 32, 32] } else { [144, 128, 32, 32] };
    out.push(sprite(Tex::Info, pad, 440, 400));
    out
}

/// The winner's header pulse: 0 up to 102 and back over 21 ticks each way.
#[derive(Default)]
struct Pulse {
    down: bool,
    c: i32,
}

impl Pulse {
    fn advance(&mut self) {
        self.c -= 1;
        if self.c < 0 {
            (self.c, self.down) = (0x14, !self.down);
        }
    }

    fn step(&mut self) -> i32 {
        let a = if self.down { self.c * 0x66 / 0x14 } else { 0x66 - self.c * 0x66 / 0x14 };
        self.advance();
        a
    }

    /// The same shape in scale: 1 up to 1.3 and back.
    fn scale(&mut self) -> f32 {
        let k = if self.down { self.c } else { 0x14 - self.c };
        self.advance();
        k as f32 * 0.015 + 1.0
    }
}

/// The sheets, and the system sounds' bank (the slide and Continue sounds).
#[derive(Resource)]
struct Art(Vec<Handle<Image>>, Option<std::sync::Arc<crate::audio::SoundBank>>);
#[derive(Component)]
struct Slot(usize);
#[derive(Component)]
pub(super) struct Root;
const POOL: usize = 400;

/// The "Game, Set, Match!" model.
#[derive(Resource)]
struct Banner(hst_sim::effect::Effect, crate::effects::Shown);
/// Its scale, the screen point it stands on (page 0 at x 0), and the ticks the ceremony before the result page
/// runs it on (seen in a recording).
const BANNER_SCALE: f32 = 1.7;
const BANNER_AT: [f32; 2] = [321.0, 53.0];
const CEREMONY: usize = 202;

#[derive(Resource, Default)]
struct Screen {
    pulse: Pulse,
    grow: Pulse,
    alpha: i32,
    scale: f32,
    acc: f32,
    open: bool,
    /// The page shown (0 the result, 1 the stats); the slide's offset, direction (1 left, 2 right) and page.
    page: usize,
    slide: i32,
    dir: u8,
    target: usize,
    /// The stats page has been shown.
    seen: bool,
    timer: i32,
    blink: i32,
}

impl Screen {
    fn opened(page: usize) -> Self {
        let p = || Pulse { down: false, c: 0x14 };
        Screen { open: true, pulse: p(), grow: p(), scale: 1.0, page, seen: page == 1, ..default() }
    }

    /// A tick with `input` (1 slides on to the next page, 2 back); whether a slide started.
    fn step(&mut self, input: u8) -> bool {
        let start = self.dir == 0 && matches!((input, self.page), (1, 0) | (2, 1));
        if start {
            (self.dir, self.target, self.slide) = (input, 1 - self.page, 0);
        }
        if self.dir != 0 {
            self.slide += if self.dir == 2 { 40 } else { -40 };
            if self.slide.abs() > 640 {
                (self.slide, self.dir, self.page) = (0, 0, self.target);
                self.seen |= self.page == 1;
            }
        }
        self.blink = (self.blink + 1) % 40;
        self.alpha = self.pulse.step();
        self.scale = self.grow.scale();
        self.timer += 1;
        start
    }

    /// Where page `p` is drawn across, if it is.
    fn page_x(&self, p: usize) -> Option<i32> {
        if self.page == p {
            Some(self.slide)
        } else if self.dir != 0 && self.target == p {
            Some(self.slide + if self.dir == 1 { 640 } else { -640 })
        } else {
            None
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Screen>()
        .add_systems(PostStartup, setup.after(super::setup))
        .add_systems(FixedUpdate, tick.after(super::simulate))
        .add_systems(Update, (step, draw, draw_banner).chain());
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    args: Res<Args>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut bindposes: ResMut<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>,
) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let mut art = Vec::new();
    for (xb, names) in [("AZUMA/PRIZE/OTHER.XB0", &TEXTURES[..]), ("AZUMA/INPANE/INPANE.XB0", &INPANE[..])] {
        let data = iso.read(xb).expect("archive on disc");
        let arc = Archive::parse(&data).expect("xb archive");
        for name in names {
            let e = arc
                .entries
                .iter()
                .find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name))
                .unwrap_or_else(|| panic!("{name} in {xb}"));
            art.push(panel::image(&mut images, &arc.read(e).expect("archive bytes")));
        }
        if xb.contains("PRIZE") {
            // game space (y down) → Bevy, as the call models
            let root = commands.spawn((Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::PI)), Visibility::default())).id();
            let (mut effect, shown) = crate::effects::model(&arc, "azuma/inpane/mdl/i_gameset_00", &mut commands, root, &mut meshes, &mut materials, &mut images, &mut bindposes)
                .expect("i_gameset_00");
            effect.hold = true;
            commands.entity(root).insert_recursive::<Children>(bevy::camera::visibility::RenderLayers::layer(popups::CALL_LAYER));
            commands.insert_resource(Banner(effect, shown));
        }
    }
    let bank = crate::audio::SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd");
    commands.insert_resource(Art(art, bank.map(std::sync::Arc::new)));
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

/// Opens the screen when a match has ended (holding the match's fixed clock) and steps it at 60 Hz: → / ← (✕ to the
/// unseen stats page) slide, ✕ from tick 120 with the stats page seen closes it.
#[allow(clippy::too_many_arguments)]
fn step(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    mut fixed: ResMut<Time<Fixed>>,
    mut g: ResMut<Game>,
    mut screen: ResMut<Screen>,
    mut pads: ResMut<Pads>,
    mut banner: Option<ResMut<Banner>>,
    sounds: Option<Res<Art>>,
    sound: Option<Res<crate::audio::Sound>>,
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
        info!("result screen opens");
        *screen = Screen::opened(std::env::var("HST_STATS_PAGE").map_or(0, |p| (p == "1") as usize));
        if let Some(b) = banner.as_mut() {
            b.0.start();
            for _ in 0..CEREMONY {
                b.0.tick();
            }
        }
        // ponytail: the fixed clock runs on but never reaches a step, as the pause menu
        fixed.set_timestep(Duration::from_secs(1 << 20));
    }
    let any = |b: GamepadButton| gamepads.iter().any(|p| p.just_pressed(b));
    let confirm = keys.any_just_pressed([KeyCode::Enter, KeyCode::Space, KeyCode::KeyJ]) || any(GamepadButton::South);
    let right = keys.any_just_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) || any(GamepadButton::DPadRight);
    let left = keys.any_just_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) || any(GamepadButton::DPadLeft);
    let play = |key: u8| {
        if let (Some(s), Some(b)) = (&sound, sounds.as_deref().and_then(|a| a.1.as_ref())) {
            s.play_centre(b, hst_sim::sound::Play { slot: 9, program: 0, key, volume: 0x80, speed: 1.0 });
        }
    };
    for s in &mut pads.slots {
        s.shot = None;
        s.serve = false;
    }
    // ponytail: the left stick's directions aren't read
    let mut input = match () {
        _ if confirm && screen.timer >= 120 => 3,
        _ if right => 1,
        _ if left => 2,
        _ if confirm && !screen.seen => 1,
        _ => 0,
    };
    if input == 3 {
        if screen.seen {
            play(2);
            g.match_stats.result = None;
            screen.open = false;
            fixed.set_timestep_hz(60.0);
            let over = fixed.overstep();
            fixed.discard_overstep(over);
            return;
        }
        input = 1;
    }
    screen.acc += time.delta_secs() * 60.0;
    while screen.acc >= 1.0 {
        screen.acc -= 1.0;
        if screen.step(input) {
            play(0xc);
        }
        input = 0;
        if let Some(b) = banner.as_mut() {
            b.0.tick();
        }
    }
}

/// The result board's quads as the stats screen draws them.
fn board(g: &Game, slots: [usize; 4], pill: [[u8; 3]; 4], winner: usize, fin: Final) -> Vec<Quad> {
    popups::final_board(g, slots, pill, winner, fin)
        .into_iter()
        .map(|q| Quad {
            tex: match q.tex {
                popups::Tex::Board(k) => Tex::Board(k),
                popups::Tex::Face(p) => Tex::Face(p),
                _ => Tex::Slot,
            },
            src: q.src,
            dst: q.dst,
            rgb: q.rgb,
            alpha: q.alpha,
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn draw(
    g: Res<Game>,
    pads: Res<Pads>,
    screen: Res<Screen>,
    art: Option<Res<Art>>,
    panel_art: Option<Res<panel::Art>>,
    board_art: Option<Res<popups::Art>>,
    colours: Option<Res<Colours>>,
    mut q: Query<(&Slot, &mut ImageNode, &mut Node, &mut Visibility)>,
) {
    let (Some(art), Some(panel_art), Some(board_art), Some(colours)) = (art, panel_art, board_art, colours) else {
        return;
    };
    let n = g.players.len();
    let slot = |i: usize| if i < n { pads.slot_of(i, n).unwrap_or(4) } else { 4 };
    let mut quads = Vec::new();
    if let Some(table) = g.match_stats.result.as_ref().filter(|_| screen.open) {
        let slots = std::array::from_fn(slot);
        // the page being left first, then the one coming in
        let mut pages = vec![screen.page];
        if screen.dir != 0 {
            pages.push(screen.target);
        }
        for p in pages {
            let Some(dx) = screen.page_x(p) else { continue };
            let mut page = if p == 0 {
                let fin = Final { highlight: screen.alpha, scale: screen.scale, dx: dx as f32 };
                board(&g, slots, colours.0, table.winner, fin)
            } else {
                let mut page = layout(&View {
                    table,
                    players: n,
                    slots,
                    pill: colours.0,
                    // COM players show their AI row's rank, as the panel; humans none
                    ranks: std::array::from_fn(|i| (i < n && slot(i) == 4).then(|| g.players[i].ai.rank as usize)),
                    rank_rgb: colours.1,
                    pulse: screen.alpha,
                });
                page.iter_mut().for_each(|q| q.dst[0] += dx as f32);
                page
            };
            quads.append(&mut page);
        }
        quads.extend(bar(screen.page, screen.timer, screen.blink));
    }
    for (Slot(i), mut img, mut node, mut vis) in &mut q {
        let Some(quad) = quads.get(*i) else {
            *vis = Visibility::Hidden;
            continue;
        };
        img.image = match quad.tex {
            Tex::Frame => art.0[0].clone(),
            Tex::Text => art.0[1].clone(),
            Tex::Keys => art.0[2].clone(),
            Tex::Back => art.0[3].clone(),
            Tex::Info => art.0[4].clone(),
            Tex::Face(p) => panel_art.0[panel::FACES + p].clone(),
            Tex::Rank => panel_art.0[6].clone(),
            Tex::Slot => panel_art.0[1].clone(),
            Tex::Board(k) => board_art.0[5 + k as usize].clone(),
        };
        let [u, v, w, h] = quad.src;
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

/// The banner over the result page, where the page is: a screen point turned into the overlay camera's plane 10 in
/// front of it.
fn draw_banner(
    g: Res<Game>,
    screen: Res<Screen>,
    banner: Option<Res<Banner>>,
    mut q: Query<(&mut Visibility, &mut bevy::mesh::morph::MorphWeights)>,
    mut joints: Query<&mut Transform>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(banner) = banner else { return };
    let (effect, shown) = (&banner.0, &banner.1);
    let x = screen.page_x(0).filter(|_| screen.open && g.match_stats.result.is_some());
    let Some(x) = x else {
        if let Ok((mut v, _)) = q.get_mut(shown.root) {
            *v = Visibility::Hidden;
        }
        return;
    };
    crate::effects::pose(effect, shown, &mut q, &mut joints, &mut materials);
    let w = 2.0 * popups::CALL_DISTANCE * popups::CALL_FOV_DEG.to_radians().tan();
    let at = Vec3::new(
        (BANNER_AT[0] + x as f32 - 320.0) * w / 640.0,
        (BANNER_AT[1] - 224.0) * w * 0.75 / 448.0,
        popups::CALL_DISTANCE,
    );
    if let Ok(mut t) = joints.get_mut(shown.root) {
        *t = Transform::from_translation(at).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)).with_scale(Vec3::splat(BANNER_SCALE));
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
        let rank_rgb = [[127, 105, 112], [82, 107, 121], [0; 3], [0; 3]];
        let pill = [[98, 46, 61], [33, 74, 97], [0; 3], [0; 3]];
        let v = View { table: &t, players: 2, slots: [0, 4, 4, 4], pill, ranks: [None, Some(5), None, None], rank_rgb, pulse: 51 };
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
        // the background at half alpha; the COM's rank under its label (the right half: at the label)
        assert_eq!((quads[0].tex, quads[0].alpha), (Tex::Back, 64.0));
        let ranks: Vec<_> = quads.iter().filter(|q| q.tex == Tex::Rank).map(|q| (q.src, q.dst, q.rgb)).collect();
        assert_eq!(ranks, vec![([0.0, 120.0, 64.0, 24.0], [410.0, 104.0, 64.0, 24.0], [82.0, 107.0, 121.0])]);
        // "Stats" from olive at the top to grey at the bottom; the row names plain
        let stats: Vec<_> = quads.iter().filter(|q| q.dst[0] == 240.0 && q.dst[1] < 120.0).collect();
        assert_eq!((stats.len(), stats[0].src, stats[39].dst[1]), (40, [0.0, 64.0, 160.0, 1.0], 119.0));
        assert!(stats[0].rgb[2] < 59.0 && stats[39].rgb[2] > 126.0);
        assert!(quads.iter().filter(|q| q.dst[0] == 240.0 && q.dst[1] >= 120.0 && q.dst[3] == 24.0).all(|q| q.rgb == WHITE));
    }

    /// The player on the left half: the rank beside the face, 4 px further left for an even player.
    #[test]
    fn rank_left() {
        let t = table();
        let v = View { table: &t, players: 2, slots: [4, 0, 4, 4], pill: [[0; 3]; 4], ranks: [Some(2), None, None, None], rank_rgb: [[1; 3]; 4], pulse: 0 };
        let q = layout(&v).into_iter().find(|q| q.tex == Tex::Rank).unwrap();
        assert_eq!((q.src, q.dst), ([0.0, 48.0, 64.0, 24.0], [(0x8a + 61 - 4) as f32, 104.0, 64.0, 24.0]));
    }

    /// → slides 40 a tick to the stats page and lands on it past 640; → again does nothing, ← slides back.
    #[test]
    fn slide() {
        let mut s = Screen::opened(0);
        assert!(s.step(1));
        assert_eq!((s.slide, s.page_x(0), s.page_x(1)), (-40, Some(-40), Some(600)));
        for _ in 0..15 {
            assert!(!s.step(0));
        }
        assert_eq!((s.slide, s.page, s.seen), (-640, 0, false));
        s.step(0);
        assert_eq!((s.slide, s.page, s.seen, s.page_x(0)), (0, 1, true, None));
        assert!(!s.step(1));
        assert!(s.step(2));
        assert_eq!((s.page_x(1), s.page_x(0)), (Some(40), Some(-600)));
        assert_eq!((s.timer, s.blink), (19, 19));
        // the scale pulse: 1 up 0.015 a tick to 1.3, then back down
        let mut s = Screen::opened(0);
        let k: Vec<f32> = (0..22).map(|_| (s.step(0), s.scale).1).collect();
        assert_eq!((k[0], k[19], k[20], k[21]), (1.0, 1.285, 1.3, 1.3));
    }

    /// Before tick 120 only the other page's name and the d-pad; then ✕ Continue, its ✕ blinking.
    #[test]
    fn bar_items() {
        let src = |q: Vec<Quad>| q.iter().map(|q| (q.tex, q.src[0] as i32, q.src[1] as i32, q.dst[0] as i32)).collect::<Vec<_>>();
        assert_eq!(src(bar(0, 119, 0)), vec![(Tex::Keys, 0, 24, 472), (Tex::Info, 192, 64, 440)]);
        assert_eq!(
            src(bar(1, 120, 20)),
            vec![(Tex::Keys, 0, 48, 288), (Tex::Info, 144, 160, 256), (Tex::Info, 224, 96, 256), (Tex::Keys, 0, 0, 472), (Tex::Info, 144, 128, 440)]
        );
    }

    #[test]
    fn doubles_fits() {
        let t = table();
        let v = View { table: &t, players: 4, slots: [0, 4, 4, 4], pill: [[128; 3]; 4], ranks: [Some(0); 4], rank_rgb: [[128; 3]; 4], pulse: 0 };
        // both pages while sliding, and the bar
        assert!(layout(&v).len() + 100 <= POOL);
    }

    #[test]
    fn pulse() {
        let mut p = Pulse { down: false, c: 0x14 };
        let a: Vec<i32> = (0..44).map(|_| p.step()).collect();
        assert_eq!((a[0], a[20], a[21], a[41], a[42]), (0, 102, 102, 0, 0));
    }
}
