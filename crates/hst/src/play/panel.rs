//! The original's player/score panel (textures from `AZUMA/INPANE/INPANE.XB0`), shown from the serve until the
//! rally starts: each player's pill, face, slot label (1P..4P / COM) and rank, both teams' points, the Sets/Games
//! strips, the team banners (doubles) and the pulsing serve ring.
//!
//! It slides in from the screen's edges over the serve's first ticks (offset `cnt·44.8 − 224` for cnt 0..5), the
//! ring pulses once it is in (31 ticks up, 31 down), and when the rally starts everything fades out over 5 ticks
//! (alpha 128·c/5, c = 5..1) except the team banners, which stay solid until the panel goes. Coordinates are the
//! PS2's 640×448 screen, stretched over the window; colours are GS colours (128 = the texture's own).

use bevy::prelude::*;
use hst_data::{iso::Iso, tim2, xb::Archive};
use hst_sim::score::{Rules, Score};

use super::{Game, Pads, Phase};
use crate::Args;

/// The panel's textures, in `Tex` order.
const TEXTURES: [&str; 11] = [
    "i_status_00",
    "inpane_p",
    "i_status_02",
    "inpane_kihontokuten00",
    "inpane_tiebreak00",
    "inpane_sen1",
    "inpane_dani",
    "inpane_mini0",
    "inpane_mini1",
    "inpane_team1",
    "i_gameinfo_00",
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tex {
    /// White pill behind a player (112×48 at 0,0).
    Pill,
    /// "1P 2P 3P 4P COM", 40×24 cells.
    Slot,
    /// The serve ring, 56×56.
    Ring,
    /// Points 0/15/30/40/Deuce/Advantage, 128×64 cells.
    Points,
    /// Tiebreak points, 64×64 cells.
    Tiebreak,
    /// The Sets/Games strip's background fade.
    Strip,
    /// Rank labels, 64×24 rows.
    Rank,
    /// "Games:" and the games digits.
    Games,
    /// "Sets:" and the sets digits.
    Sets,
    /// Team banner text and its pill pieces.
    Team,
    /// "Score to win this game!" and the other calls, 256×24 rows.
    Info,
    /// Player `i`'s face, 64×64.
    Face(usize),
}

/// One textured rectangle: source rect in the texture (u, v, w, h), screen rect (x, y, w, h), GS tint and alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Quad {
    tex: Tex,
    src: [f32; 4],
    dst: [f32; 4],
    rgb: [f32; 3],
    alpha: f32,
}

/// What the panel shows this frame.
struct View {
    players: usize,
    /// Per player: 0..3 the controlling slot (1P..4P), 4 COM.
    slots: [usize; 4],
    /// Per player: rank label row (none for humans).
    ranks: [Option<usize>; 4],
    pill: [[u8; 3]; 4],
    rank_rgb: [[u8; 3]; 4],
    points: [i32; 2],
    games: [i32; 2],
    sets: [i32; 2],
    deuce: bool,
    advantage: bool,
    tiebreak: bool,
    server: i32,
    /// 0 deuce court, 1 ad court.
    court: i32,
    /// The first team's panel is at the bottom (its end is nearer the camera).
    first_near: bool,
    /// The "Score to win" line: the team it speaks to and its row (see `game_info`).
    info: Option<(usize, usize)>,
    clock: Clock,
}

/// The panel's own timeline, counted in match ticks.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Clock {
    /// Ticks since the serve phase began (1 on its first tick), 0 when not serving.
    serve: u32,
    /// Ticks since the rally began, while the panel fades out (0: not fading).
    fade: u32,
}

impl Clock {
    fn step(&mut self, phase: &Phase) {
        *self = match phase {
            Phase::Serve => Clock {
                serve: self.serve + 1,
                fade: 0,
            },
            // the fade runs to 6 (alpha 0), then the panel is gone
            Phase::Rally if self.serve > 0 && self.fade < 6 => Clock {
                serve: self.serve,
                fade: self.fade + 1,
            },
            _ => Clock::default(),
        };
    }

    fn shown(&self) -> bool {
        self.serve > 0
    }

    /// Slide-in count, 0..5 (5 = fully in).
    fn cnt(&self) -> i32 {
        (self.serve as i32 - 1).min(5)
    }

    /// The faded textures' alpha (also the strip's base level).
    fn alpha(&self) -> i32 {
        if self.fade == 0 {
            128
        } else {
            (6 - self.fade as i32) * 128 / 5
        }
    }

    /// The serve ring's alpha once the panel is in: up over 31 ticks, down over 31.
    fn ring(&self) -> i32 {
        let k = (self.serve as i32 - 7).rem_euclid(62);
        if k < 31 {
            128 - (30 - k) * 128 / 30
        } else {
            (61 - k) * 128 / 30
        }
    }
}

/// Points cells (column, row) by point index: 0, 15, 30, 40, Ad (→ Deuce/Advantage handled apart).
const POINT_U: [i32; 7] = [0, 1, 0, 1, 0, 1, 0];
const POINT_V: [i32; 7] = [0, 0, 1, 1, 2, 2, 3];
const TIEBREAK_U: [i32; 11] = [0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 0];
const TIEBREAK_V: [i32; 11] = [0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3];

fn layout(v: &View) -> Vec<Quad> {
    let mut out = Vec::new();
    if !v.clock.shown() || v.players < 2 {
        return out;
    }
    let n = v.players;
    let a = v.clock.alpha() as f32;
    let fading = v.clock.fade > 0;
    let mut q =
        |tex, src: [i32; 4], x: i32, y: i32, size: Option<[i32; 2]>, rgb: [f32; 3], alpha: f32| {
            let [w, h] = size.unwrap_or([src[2], src[3]]);
            out.push(Quad {
                tex,
                src: src.map(|s| s as f32),
                dst: [x, y, w, h].map(|s| s as f32),
                rgb,
                alpha,
            })
        };
    const WHITE: [f32; 3] = [128.0; 3];
    let rgb = |c: [u8; 3]| c.map(f32::from);

    let i8 = if n < 3 { 20 } else { 0 };
    let tb_index = |p: i32| (p.max(0) as usize).min(TIEBREAK_U.len() - 1);
    let pt_index = |p: i32| (p.max(0) as usize).min(POINT_U.len() - 1);
    // digit offset and second team's shift: only with deuce, advantage or a tiebreak
    let (i24, mut i26) = match (v.tiebreak, v.deuce, v.advantage) {
        (false, false, false) => (0, 0),
        (false, true, _) | (true, true, _) => (8, -24),
        (false, false, true) => (8, 0),
        (true, false, false) => (16, 32),
        (true, false, true) => (8, 32),
    };
    let off = (v.clock.cnt() as f32 * 44.8 - 224.0) as i32;
    let fully_in = v.clock.cnt() > 4;
    let ys = if v.first_near { [328, 40] } else { [40, 328] };
    let server_team = (v.server & 1) as usize;
    let mirrored = (v.first_near && v.court == 1) || (!v.first_near && v.court == 0);
    let adv_lead = |t: usize| v.advantage && v.points[t] > v.points[1 - t];

    let mut face = [[0; 2]; 4];
    let mut label = [[0; 2]; 4];
    let (mut i27, mut i28) = (0, 0);
    // the team drawn on the left: the second when mirrored
    let left = if mirrored { 1 } else { 0 };
    if adv_lead(1 - left) {
        i26 = -24;
    }
    let right_x = |base: i32| i26 + base - off - i24;
    let (lt, rt) = (left, 1 - left);
    if n < 3 {
        face[lt] = [off + 16, ys[lt] + i8];
        label[lt] = [off + 48, ys[lt] + i8];
        face[rt] = [right_x(424), ys[rt] + i8];
        label[rt] = [right_x(456), ys[rt] + i8];
    } else {
        // the team's first player sits low (y + 40), its partner high, 16 further out
        for (t, x0, x1) in [(lt, off + 32, off + 64), (rt, right_x(440), right_x(472))] {
            face[t] = [x0, ys[t] + 40];
            label[t] = [x1, ys[t] + 40];
            face[t + 2] = [x0 - 16, ys[t] + i8];
            label[t + 2] = [x1 - 16, ys[t] + i8];
        }
        if v.server < 2 {
            (i27, i28) = (16, 40);
        }
    }
    let mut digit_x = [0; 2];
    digit_x[lt] = off + 112 + i24;
    digit_x[rt] = i26 + 520 - off;
    let ring_shift = if server_team == lt { 0 } else { i26 };
    let mut ring_x = if mirrored {
        i27 + (ring_shift + 424 - server_team as i32 * 408) - 8
    } else {
        i27 + (ring_shift + 408) * server_team as i32 + 16 - 8
    };
    if ring_x > 319 {
        ring_x -= i24;
    }
    let ring_y = i28 + i8 + ys[server_team] - 8;
    let mut strip_x = [0; 2];
    strip_x[lt] = off + 16;
    strip_x[rt] = 440 - off;
    let strip_y = if v.first_near { [408, 14] } else { [14, 408] };

    // players: pill, face, slot label, rank
    for i in 0..n {
        let [fx, fy] = face[i];
        let [lx, ly] = label[i];
        q(Tex::Pill, [0, 0, 112, 48], fx, fy, None, rgb(v.pill[i]), a);
        q(Tex::Face(i), [0, 0, 64, 64], fx, fy, None, WHITE, a);
        q(
            Tex::Slot,
            [v.slots[i] as i32 * 40, 0, 40, 24],
            lx,
            ly,
            None,
            rgb(v.pill[i]),
            a,
        );
        if let Some(r) = v.ranks[i] {
            q(
                Tex::Rank,
                [0, r as i32 * 24, 64, 24],
                lx + 8,
                ly + 24,
                None,
                rgb(v.rank_rgb[i]),
                a,
            );
        }
    }
    // points; with advantage the trailing team is drawn at half alpha
    let half = if fading { (a as i32 / 2) as f32 } else { 64.0 };
    for t in 0..2 {
        let (x, y) = (digit_x[t], ys[t] + 8);
        let p = v.points[t];
        let (tex, src, alpha) = match (v.tiebreak, v.deuce, v.advantage) {
            (false, true, _) => (Tex::Points, [0, 128, 128, 64], a),
            (true, true, _) => (Tex::Tiebreak, [0, 128, 128, 64], a),
            (false, false, true) if p == 4 => (Tex::Points, [128, 128, 128, 64], a),
            (true, false, true) if p == 8 => (Tex::Tiebreak, [128, 128, 128, 64], a),
            (false, false, adv) => {
                let k = pt_index(p);
                (
                    Tex::Points,
                    [POINT_U[k] * 128, POINT_V[k] * 64, 128, 64],
                    if adv { half } else { a },
                )
            }
            (true, false, adv) => {
                let k = tb_index(p);
                (
                    Tex::Tiebreak,
                    [TIEBREAK_U[k] * 64, TIEBREAK_V[k] * 64, 64, 64],
                    if adv { half } else { a },
                )
            }
        };
        q(tex, src, x, y, None, WHITE, alpha);
    }
    // the Sets/Games strips, then (doubles) the team banners
    for t in 0..2 {
        let (x, y) = (strip_x[t], strip_y[t]);
        let (bg_y, banner_y) = if y < 224 { (16, 122) } else { (410, 298) };
        let right = x > 319;
        let (bg_x, banner_x) = if right {
            (400 - off, 528 - off)
        } else {
            (off, off)
        };
        let sa = (a * 0.35).trunc();
        if right {
            q(
                Tex::Strip,
                [0, 0, 40, 16],
                bg_x,
                bg_y,
                Some([40, 20]),
                WHITE,
                sa,
            );
            q(
                Tex::Strip,
                [40, 0, 16, 16],
                bg_x + 40,
                bg_y,
                Some([200, 20]),
                WHITE,
                sa,
            );
        } else {
            q(
                Tex::Strip,
                [40, 0, 16, 16],
                bg_x,
                bg_y,
                Some([200, 20]),
                WHITE,
                sa,
            );
            q(
                Tex::Strip,
                [56, 0, 40, 16],
                bg_x + 200,
                bg_y,
                Some([40, 20]),
                WHITE,
                sa,
            );
        }
        q(Tex::Sets, [0, 0, 72, 24], x, y, None, WHITE, a);
        q(
            Tex::Sets,
            [v.sets[t] * 16, 24, 16, 24],
            x + 72,
            y,
            None,
            WHITE,
            a,
        );
        q(Tex::Games, [0, 0, 72, 24], x + 96, y, None, WHITE, a);
        q(
            Tex::Games,
            [v.games[t] * 16, 24, 16, 24],
            x + 168,
            y,
            None,
            WHITE,
            a,
        );
        // the team the "Score to win" line speaks to loses its banner while the line is up
        if n > 2 && !(fully_in && v.info.is_some_and(|(a, _)| a == t)) {
            let tint = if t == 0 {
                [127.0, 77.0, 77.0]
            } else {
                [67.0, 101.0, 127.0]
            };
            if right {
                q(
                    Tex::Team,
                    [88, 0, 16, 24],
                    banner_x,
                    banner_y,
                    None,
                    tint,
                    128.0,
                );
                q(
                    Tex::Team,
                    [104, 0, 8, 24],
                    banner_x + 16,
                    banner_y,
                    Some([96, 24]),
                    tint,
                    128.0,
                );
            } else {
                q(
                    Tex::Team,
                    [104, 0, 8, 24],
                    banner_x,
                    banner_y,
                    Some([96, 24]),
                    tint,
                    128.0,
                );
                q(
                    Tex::Team,
                    [112, 0, 16, 24],
                    banner_x + 96,
                    banner_y,
                    None,
                    tint,
                    128.0,
                );
            }
            q(
                Tex::Team,
                [0, t as i32 * 24, 80, 24],
                banner_x + 16,
                banner_y,
                None,
                WHITE,
                128.0,
            );
        }
    }
    if fully_in && !fading {
        q(
            Tex::Ring,
            [0, 0, 56, 56],
            ring_x,
            ring_y,
            None,
            WHITE,
            v.clock.ring() as f32,
        );
    }
    // the "Score to win" line on its team's side, just inside its panel (y 300 bottom, 124 top); it blinks (15 ticks off, 15
    // on, from when the panel is in) and fades out solid with the panel
    if let Some((t, row)) = v.info.filter(|_| fully_in) {
        let x = if t == left { 0 } else { 384 };
        let y = if ys[t] > 200 { 300 } else { 124 };
        let blink = if (v.clock.serve as i32 - 7).rem_euclid(30) < 15 { 0.0 } else { 128.0 };
        q(Tex::Info, [0, row as i32 * 24, 256, 24], x, y, None, WHITE, if fading { a } else { blink });
    }
    out
}

/// The "Score to win" line for a game point (the first team's first): the team it speaks to and the
/// `i_gameinfo_00` row — 0 "…this game!" (the server's), 3 "…for a service break!", 1 "…the set!", 2 "…the
/// match!", or 4 "Score or you lose the match!" to the other team when only it has a human player.
fn game_info(s: &Score, r: &Rules, human: [bool; 2]) -> Option<(usize, usize)> {
    let t = (0..2).find(|&t| s.game_point(r, t))?;
    let lead = (s.games[t] > s.games[t ^ 1]) as i32;
    Some(if !s.tiebreak && s.games[t] < r.games - lead {
        (t, if (s.server & 1) as usize == t { 0 } else { 3 })
    } else if s.sets[t] < r.sets - 1 {
        (t, 1)
    } else if !human[t] && human[t ^ 1] {
        (t ^ 1, 4)
    } else {
        (t, 2)
    })
}

/// The panel's textures: `TEXTURES` in order, then each player's face.
#[derive(Resource)]
pub(super) struct Art(pub(super) Vec<Handle<Image>>);
/// Where the faces start in `Art`.
pub(super) const FACES: usize = TEXTURES.len();
/// The pool of screen rectangles, drawn in order.
#[derive(Component)]
pub(super) struct Slot(usize);
/// Per-player colours from the game program: pill/label tint and rank tint.
#[derive(Resource)]
pub(super) struct Colours(pub(super) [[u8; 3]; 4], pub(super) [[u8; 3]; 4]);

const POOL: usize = 48;

pub fn plugin(app: &mut App) {
    app.init_resource::<Clock>()
        .add_systems(PostStartup, setup.after(super::setup))
        .add_systems(FixedUpdate, tick.after(super::simulate))
        .add_systems(Update, draw);
}

/// An INPANE sheet (alpha authored 0–255: the board's 0x80 centre shows the court through it on the PS2).
pub(super) fn image(images: &mut Assets<Image>, data: &[u8]) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let pic = tim2::decode_alpha8(data).expect("TIM2").remove(0);
    let img = Image::new(
        Extent3d {
            width: pic.width,
            height: pic.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pic.rgba.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    crate::textures::add_tim2(images, img, &pic)
}

fn setup(mut commands: Commands, args: Res<Args>, g: Res<Game>, mut images: ResMut<Assets<Image>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso
        .read("AZUMA/INPANE/INPANE.XB0")
        .expect("INPANE archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let mut get = |name: String| {
        let e = arc
            .entries
            .iter()
            .find(|e| {
                e.name
                    .to_ascii_lowercase()
                    .replace('\\', "/")
                    .ends_with(&name)
            })
            .unwrap_or_else(|| panic!("{name} in INPANE"));
        image(&mut images, &arc.read(e).expect("INPANE bytes"))
    };
    let mut art: Vec<_> = TEXTURES
        .iter()
        .map(|t| get(format!("/{}.tm2", t.to_ascii_lowercase())))
        .collect();
    art.extend(g.chars.iter().enumerate().map(|(i, &c)| {
        let o = args.outfits.get(i).copied().unwrap_or(0);
        get(format!("face/face_{c:02}_{o:02}.tm2"))
    }));
    commands.insert_resource(Art(art));
    let (cnf, bin) = (
        iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"),
        iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"),
    );
    let (pill, rank) = hst_data::exe::Game::new(&cnf, &bin)
        .expect("supported disc")
        .hud_colours();
    commands.insert_resource(Colours(pill, rank));
    commands
        .spawn(super::widescreen::screen_43())
        .with_children(|p| {
            for i in 0..POOL {
                p.spawn((
                    Slot(i),
                    ImageNode { image_mode: NodeImageMode::Stretch, ..default() },
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    Visibility::Hidden,
                ));
            }
        });
}

fn tick(g: Res<Game>, mut clock: ResMut<Clock>) {
    clock.step(&g.phase);
}

fn draw(
    g: Res<Game>,
    pads: Res<Pads>,
    clock: Res<Clock>,
    art: Option<Res<Art>>,
    colours: Option<Res<Colours>>,
    cam: Query<&Transform, With<crate::Orbit>>,
    mut q: Query<(&Slot, &mut ImageNode, &mut Node, &mut Visibility)>,
) {
    let (Some(art), Some(colours)) = (art, colours) else {
        return;
    };
    let n = g.players.len();
    let s = &g.score;
    // the first team's end is game −z when its `end` is +1, world +z; it is near when the camera is on that side
    let first_near = cam
        .single()
        .map_or(true, |c| c.translation.z * g.players[0].end >= 0.0);
    let slot = |i: usize| pads.slot_of(i, n).unwrap_or(4);
    let view = View {
        players: n,
        slots: std::array::from_fn(|i| if i < n { slot(i) } else { 4 }),
        // ponytail: COM players show rank row 0 ("Lv 5"); the original takes it from the opponent's profile
        ranks: std::array::from_fn(|i| (i < n && slot(i) == 4).then_some(0)),
        pill: colours.0,
        rank_rgb: colours.1,
        points: s.points,
        games: s.games,
        sets: s.sets,
        deuce: s.deuce,
        advantage: s.advantage,
        tiebreak: s.tiebreak,
        server: s.server,
        court: s.side,
        first_near,
        info: game_info(s, &g.rules, [0, 1].map(|t| (t..n).step_by(2).any(|i| slot(i) < 4))),
        clock: *clock,
    };
    let quads = layout(&view);
    for (Slot(i), mut img, mut node, mut vis) in &mut q {
        let Some(quad) = quads.get(*i) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let tex = match quad.tex {
            Tex::Face(p) => TEXTURES.len() + p,
            t => [
                Tex::Pill,
                Tex::Slot,
                Tex::Ring,
                Tex::Points,
                Tex::Tiebreak,
                Tex::Strip,
                Tex::Rank,
                Tex::Games,
                Tex::Sets,
                Tex::Team,
                Tex::Info,
            ]
            .iter()
            .position(|&k| k == t)
            .unwrap(),
        };
        let [u, v, w, h] = quad.src;
        img.image = art.0[tex].clone();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn doubles(clock: Clock) -> View {
        View {
            players: 4,
            slots: [4; 4],
            ranks: [Some(0), Some(0), Some(0), Some(1)],
            pill: [[98, 46, 61], [33, 74, 97], [105, 70, 25], [63, 78, 25]],
            rank_rgb: [[0; 3]; 4],
            points: [0, 0],
            games: [0, 0],
            sets: [0, 0],
            deuce: false,
            advantage: false,
            tiebreak: false,
            server: 0,
            court: 0,
            first_near: true,
            info: None,
            clock,
        }
    }

    /// Slot 5 of the reference saves (doubles, all COM, first team near, deuce court, player 0 serving): the
    /// positions read off the original's draw and matched against its screenshot.
    #[test]
    fn doubles_in() {
        let quads = layout(&doubles(Clock { serve: 40, fade: 0 }));
        let at = |tex: Tex| {
            quads
                .iter()
                .filter(|q| q.tex == tex)
                .map(|q| [q.dst[0] as i32, q.dst[1] as i32])
                .collect::<Vec<_>>()
        };
        assert_eq!(at(Tex::Face(0)), [[32, 368]]);
        assert_eq!(at(Tex::Face(2)), [[16, 328]]);
        assert_eq!(at(Tex::Face(1)), [[440, 80]]);
        assert_eq!(at(Tex::Face(3)), [[424, 40]]);
        assert_eq!(at(Tex::Slot), [[64, 368], [472, 80], [48, 328], [456, 40]]);
        assert_eq!(at(Tex::Points), [[112, 336], [520, 48]]);
        assert_eq!(at(Tex::Ring), [[24, 360]]);
        assert_eq!(
            &at(Tex::Sets)[..],
            &[[16, 408], [88, 408], [440, 14], [512, 14]]
        );
    }

    /// The stretched background pieces (B5): each piece's sub-rect and on-screen size as the original's draw passes
    /// them (left strip at the bottom, right strip at the top, banners above/below them).
    #[test]
    fn doubles_pieces() {
        let quads = layout(&doubles(Clock { serve: 40, fade: 0 }));
        let pieces = |tex: Tex| {
            quads
                .iter()
                .filter(|q| q.tex == tex && (tex != Tex::Team || q.rgb != [128.0; 3]))
                .map(|q| (q.src.map(|s| s as i32), q.dst.map(|s| s as i32)))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            pieces(Tex::Strip),
            [
                ([40, 0, 16, 16], [0, 410, 200, 20]),
                ([56, 0, 40, 16], [200, 410, 40, 20]),
                ([0, 0, 40, 16], [400, 16, 40, 20]),
                ([40, 0, 16, 16], [440, 16, 200, 20]),
            ]
        );
        assert_eq!(
            pieces(Tex::Team),
            [
                ([104, 0, 8, 24], [0, 298, 96, 24]),
                ([112, 0, 16, 24], [96, 298, 16, 24]),
                ([88, 0, 16, 24], [528, 122, 16, 24]),
                ([104, 0, 8, 24], [544, 122, 96, 24]),
            ]
        );
    }

    #[test]
    fn timeline() {
        let mut c = Clock::default();
        let mut cnt = Vec::new();
        for _ in 0..7 {
            c.step(&Phase::Serve);
            cnt.push(c.cnt());
        }
        assert_eq!(cnt, [0, 1, 2, 3, 4, 5, 5]);
        assert_eq!(c.ring(), 0);
        let mut alpha = Vec::new();
        for _ in 0..7 {
            c.step(&Phase::Rally);
            alpha.push((c.shown(), c.alpha()));
        }
        assert_eq!(
            alpha,
            [
                (true, 128),
                (true, 102),
                (true, 76),
                (true, 51),
                (true, 25),
                (true, 0),
                (false, 128)
            ]
        );
    }

    #[test]
    fn score_to_win() {
        let r = Rules { sets: 2, games: 6, no_deuce: false, one_point_games: false, players: 4 };
        let mut s = Score { points: [3, 1], ..default() };
        assert_eq!(game_info(&s, &r, [false; 2]), Some((0, 0)));
        s.server = 1;
        assert_eq!(game_info(&s, &r, [false; 2]), Some((0, 3)));
        s.games = [5, 3];
        assert_eq!(game_info(&s, &r, [false; 2]), Some((0, 1)));
        s.sets = [1, 0];
        assert_eq!(game_info(&s, &r, [false; 2]), Some((0, 2)));
        assert_eq!(game_info(&s, &r, [false, true]), Some((1, 4)));
        s.deuce = true;
        assert_eq!(game_info(&s, &r, [false; 2]), None);

        // the first team near (bottom, left): its line on the left, just above its panel; blinking in once the panel is in
        let mut c = Clock::default();
        let mut v = doubles(c);
        v.info = Some((0, 3));
        let line = |v: &View| layout(v).into_iter().find(|q| q.tex == Tex::Info).map(|q| (q.src, q.dst, q.alpha));
        assert_eq!(line(&v), None);
        let mut alpha = Vec::new();
        for _ in 0..37 {
            c.step(&Phase::Serve);
            v.clock = c;
            alpha.push(line(&v).map(|l| l.2));
        }
        assert_eq!(alpha[..6], [None, None, None, None, None, Some(128.0)]);
        assert_eq!(alpha[6..21], [Some(0.0); 15]);
        assert_eq!(alpha[21..36], [Some(128.0); 15]);
        let l = line(&v).unwrap();
        assert_eq!((l.0, l.1), ([0.0, 72.0, 256.0, 24.0], [0.0, 300.0, 256.0, 24.0]));
        v.info = Some((1, 0));
        assert_eq!(line(&v).unwrap().1, [384.0, 124.0, 256.0, 24.0]);
        // the line's team has no banner (the other's is still there)
        let banners = |v: &View| layout(v).iter().filter(|q| q.tex == Tex::Team && q.rgb == [128.0; 3]).map(|q| q.src[1]).collect::<Vec<_>>();
        assert_eq!(banners(&v), [0.0]);
    }
}
