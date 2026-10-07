//! The original's score pop-up after a point (textures from `AZUMA/INPANE/INPANE.XB0`), drawn from the score show
//! that `hst_sim::flow` runs: both players' plates (pill, face, slot label, rank) beside both teams' points, the
//! scorer's old points squeezed out to the left while the new ones grow in from the right (5 steps of 25.6 px), a
//! white copy of the new points flashed on top and faded out over 15 ticks, the hold, then the whole thing fading out
//! over 5 ticks (alpha 128·t/5). At deuce it is the "Deuce!" banner instead, with the deuce count (×N) from the
//! second deuce on, squashed and restored as the show rolls. Coordinates are the PS2's 640×448 screen, as the panel.

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
}

/// Points cells (column, row) by point index; 4 is Deuce, 5 Advantage.
const POINT_U: [i32; 7] = [0, 1, 0, 1, 0, 1, 0];
const POINT_V: [i32; 7] = [0, 0, 1, 1, 2, 2, 3];

fn layout(v: &View) -> Vec<Quad> {
    let mut out = Vec::new();
    let st = v.show;
    // ponytail: game/set/tiebreak shows are P12b2
    if st.event != Event::Point || v.players < 2 {
        return out;
    }
    let mut q = |tex, src: [f32; 4], dst: [f32; 4], rgb: [f32; 3], alpha: i32| {
        if dst[2] > 0.0 && dst[3] > 0.0 && alpha > 0 {
            out.push(Quad { tex, src, dst, rgb, alpha: alpha as f32 })
        }
    };
    const WHITE: [f32; 3] = [128.0; 3];
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
        q(Tex::Deuce, [0.0, 0.0, 256.0, 64.0], [192.0, 192.0, 256.0, 64.0], WHITE, a);
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
                    Tex::Deuce,
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
        return out;
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
    out
}

/// kihontokuten01 (white points) and duce00 ("Deuce!").
#[derive(Resource)]
struct Art([Handle<Image>; 2]);
#[derive(Component)]
struct Slot(usize);

const POOL: usize = 20;

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, setup.after(super::setup)).add_systems(Update, draw);
}

fn setup(mut commands: Commands, args: Res<Args>, mut images: ResMut<Assets<Image>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso.read("AZUMA/INPANE/INPANE.XB0").expect("INPANE archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let art = ["/inpane_kihontokuten01.tm2", "/inpane_duce00.tm2"].map(|name| {
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
                p.spawn((Slot(i), ImageNode::default(), Node { position_type: PositionType::Absolute, ..default() }, Visibility::Hidden));
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
    cam: Query<&Transform, With<Camera3d>>,
    mut first_near: Local<Option<bool>>,
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
    let quads = match g.post.as_ref().and_then(|p| p.show()) {
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
            })
        }
        None => Vec::new(),
    };
    for (Slot(i), mut img, mut node, mut vis) in &mut q {
        let Some(quad) = quads.get(*i) else {
            *vis = Visibility::Hidden;
            continue;
        };
        // the panel's texture order: pill, slot, …, points (3), …, rank (6), …, faces from 10
        let handle = match quad.tex {
            Tex::Pill => &panel_art.0[0],
            Tex::Slot => &panel_art.0[1],
            Tex::Points => &panel_art.0[3],
            Tex::Rank => &panel_art.0[6],
            Tex::Face(p) => &panel_art.0[10 + p],
            Tex::PointsWhite => &art.0[0],
            Tex::Deuce => &art.0[1],
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

#[cfg(test)]
mod tests {
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
}
