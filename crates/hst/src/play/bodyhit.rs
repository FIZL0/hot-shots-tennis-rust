//! The ball hitting a player (`hst_sim::bodyhit`): each rally tick before the point is judged, every player not
//! swinging is tested against the ball with its head and trunk bones as posed; the first hit ends the point and
//! pops up CONK or SMACK at the ball, a camera-facing quad hanging up from its anchor, until the players react.
//! The ball comes back off the player (`Flight::step_plane`), and the player turns to face where it came from,
//! cries out and plays motion 0x2b, standing in it until the point's reaction.

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::bodyhit::{self, BounceWords, Popup, Word};
use hst_sim::player::ReachStats;
use hst_sim::pose::Nodes;

use super::{Figure, Game, Phase};
use crate::Args;
use crate::character::Motion;

/// Per player its character's collision size (m); the point's pop-up, and whether it has been made; the ball's
/// position a tick ago (its last move is the way it came); the pop-up's alpha a tick ago (drawn between the two).
#[derive(Resource, Clone)]
pub(super) struct Hits {
    radius: Vec<f32>,
    pop: Option<Popup>,
    made: bool,
    last: [f32; 3],
    alpha: f32,
}

/// The pop-up quad (its own material) and the two words' textures.
#[derive(Component)]
struct View(Handle<StandardMaterial>, [Handle<Image>; 2]);

pub fn plugin(app: &mut App) {
    app.init_resource::<Posed>()
        .add_systems(PostStartup, setup.after(super::setup))
        .add_systems(FixedUpdate, (detect.before(super::simulate), place_ball.before(super::simulate), (age, bounce_words).after(super::simulate), pose.after(crate::character::tick)))
        .add_systems(Update, (draw, draw_words).after(super::balloons));
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    g: Res<Game>,
    match_mod: Option<Res<crate::mods::MatchMod>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    // `HST_BODY_SIZE=<m>`: every player's collision size (research/bodyhit_rec.py pokes the game's the same way)
    let size = std::env::var("HST_BODY_SIZE").ok().and_then(|v| v.parse::<f32>().ok());
    let radius = g.chars.iter().enumerate().map(|(i, &c)| size.unwrap_or_else(|| ReachStats::from_tparam(&super::mod_match::row(&mut iso, match_mod.as_deref(), i, c as usize).join(",")).collision)).collect();
    commands.insert_resource(Hits { radius, pop: None, made: false, last: [0.0; 3], alpha: 0.0 });
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").expect("EFFCT archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let words = [Word::Conk, Word::Smack].map(|w| {
        let name = format!("panel/{}", w.texture());
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(&name)).expect("word in EFFCT");
        super::panel::image(&mut images, &arc.read(e).expect("word bytes"))
    });
    // the bottom edge is the anchor
    let quad = meshes.add(Rectangle::new(1.0, 1.0).mesh().build().translated_by(Vec3::Y * 0.5));
    let m = materials.add(StandardMaterial {
        base_color_texture: Some(words[0].clone()),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((View(m.clone(), words), Mesh3d(quad.clone()), MeshMaterial3d(m), Transform::default(), Visibility::Hidden));
    // the bounce words: one quad per word a point can make
    let textures: Vec<_> = (1..=15)
        .map(|k| {
            let name = format!("panel/cf00-{}.tm2", bodyhit::bounce_texture(k).unwrap());
            let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(&name)).expect("bounce word in EFFCT");
            super::panel::image(&mut images, &arc.read(e).expect("word bytes"))
        })
        .collect();
    commands.insert_resource(Words { words: BounceWords::default(), on: false, textures });
    for i in 0..10 {
        let m = materials.add(StandardMaterial { unlit: true, alpha_mode: AlphaMode::Blend, double_sided: true, cull_mode: None, ..default() });
        commands.spawn((WordView(i, m.clone()), Mesh3d(quad.clone()), MeshMaterial3d(m), Transform::default(), Visibility::Hidden));
    }
}

/// Swinging, diving or whiffing players aren't tested (the game's stroke state).
fn detect(mut g: ResMut<Game>, mut hits: ResMut<Hits>, posed: Res<Posed>, q: Query<&Figure, With<Motion>>) {
    let last = std::mem::replace(&mut hits.last, g.flight.ball.pos);
    if g.phase == Phase::Serve {
        g.body_hit = None;
    }
    if g.phase != Phase::Rally || g.shots == 0 || g.body_hit.is_some() {
        return;
    }
    // `HST_BODY_HIT=<player>`: that player counts as hit 30 frames into the first rally (tools/record_bodyhit.py forces
    // the same in the original)
    if let Some(i) = std::env::var("HST_BODY_HIT").ok().and_then(|v| v.parse().ok()) {
        if g.since_hit == 30 {
            g.body_hit = Some(i);
        }
        return;
    }
    let ball = g.flight.ball.pos;
    for f in &q {
        let (p, data) = (&g.players[f.0], &g.data[f.0]);
        if p.swing.is_some() || p.wait_swing.is_some() || p.dive.is_some() || p.whiff.is_some() {
            continue;
        }
        let sk = &data.skeleton;
        let bone = |n: &str| sk.names.iter().position(|b| b == n);
        let (Some(locals), Some(head), Some(neck), Some(spine)) = (posed.locals(f.0), bone("Bip01Head"), bone("Bip01Neck"), bone("Bip01Spine")) else {
            continue;
        };
        let player = super::player_matrix(p);
        let at = |n: usize| hst_sim::pose::node_world(sk, locals, n, &player);
        let point = |n: usize| {
            let r = at(n)[3];
            [r[0], r[1], r[2]]
        };
        if bodyhit::hit(ball, &at(head), point(spine), point(neck), hits.radius[f.0]) {
            info!("ball hit player {}", f.0);
            g.body_hit = Some(f.0 as i32);
            struck(&mut g, f.0, last);
            return;
        }
    }
}

/// `HST_BALL=x,y,z,vx,vy,vz` (game space): the ball is put there 30 frames into each rally, for `--shot` checks of
/// a net touch's word against `research/b85_net_rec.py` (which overwrites the original's ball the same way).
fn place_ball(mut g: ResMut<Game>) {
    let Some(v) = std::env::var("HST_BALL").ok().map(|e| e.split(',').filter_map(|x| x.trim().parse::<f32>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 6) else { return };
    if g.phase == Phase::Rally && g.since_hit == 30 {
        (g.flight.ball.pos, g.flight.ball.vel) = ([v[0], v[1], v[2]], [v[3], v[4], v[5]]);
        info!("HST_BALL: ball placed");
    }
}

/// Per player (by figure) the clip bound to its nodes and the nodes as the motion player last posed them.
#[derive(Resource, Default)]
pub(super) struct Posed(Vec<Option<(usize, Nodes)>>);

impl Posed {
    /// Player `i`'s nodes' local matrices as of this tick's motion step.
    pub(super) fn locals(&self, i: usize) -> Option<&[[[f32; 4]; 4]]> {
        Some(&self.0.get(i)?.as_ref()?.1.local)
    }
}

/// One frame of the motion player after the motions step: a new motion binds its clip (rows back to rest); the
/// clip is sampled at its time, and mid-crossfade the other motion is mixed over the first 23 tracks by this
/// tick's weight from each node's held value (as `character::animate` draws it). The held fade samples the frozen
/// outgoing pose and mixes the new motion's frame 0 over it.
// ponytail: a same-motion restart through the point's reset to no motion doesn't rebind (only a change of motion does)
pub(super) fn pose(mut posed: ResMut<Posed>, g: Res<Game>, q: Query<(&Figure, &Motion)>) {
    for (f, m) in &q {
        let Some(data) = g.data.get(f.0) else { continue };
        let (sk, Some(new)) = (&data.skeleton, data.motions.get(&m.id)) else { continue };
        if posed.0.len() <= f.0 {
            posed.0.resize(f.0 + 1, None);
        }
        let slot = &mut posed.0[f.0];
        if slot.as_ref().is_none_or(|(_, n)| n.local.len() != sk.rest.len()) {
            *slot = Some((usize::MAX, Nodes::new(sk)));
        }
        let (bound, nodes) = slot.as_mut().unwrap();
        if *bound != m.id {
            *bound = m.id;
            nodes.bind(sk, new);
        }
        let old = data.motions.get(&(m.fade.id as usize)).filter(|_| m.fade.clip);
        match (m.mix, old) {
            (Some((w, true)), Some(old)) => {
                nodes.sample(old, m.fade.sampled);
                nodes.mix(new, 0.0, w, 23);
            }
            (Some((w, false)), Some(old)) => {
                nodes.sample(new, m.clock.sampled);
                nodes.mix(old, m.fade.sampled, w, 23);
            }
            _ => nodes.sample(new, m.clock.sampled),
        }
    }
}

/// The hit as the original's handler takes it: the ball's extra step back off the player; the player faces against
/// the ball's last move (normalised flat), plays motion 0x2b once and cries out.
fn struck(g: &mut Game, i: usize, last: [f32; 3]) {
    use hst_sim::ps2::{div, madd, mul, sqrt, sub};
    let (shot, surface, material) = (g.shot, &hst_sim::ball::COURTS[g.court], g.world.1[0]);
    let d = [0, 1, 2].map(|k| sub(g.flight.ball.pos[k], last[k]));
    let before = g.flight.ball.vel;
    g.flight.step_plane(&shot, surface, material);
    info!("ball off player {i}: velocity {before:?} -> {:?}, bounces {}", g.flight.ball.vel, g.flight.bounces);
    let inv = div(1.0, sqrt(madd(madd(mul(0.0, 0.0), d[0], d[0]), d[2], d[2])));
    let dir = [-mul(d[0], inv), -mul(0.0, inv), -mul(d[2], inv), -mul(0.0, inv)];
    let p = &mut g.players[i];
    (p.body.face.dir, p.body.target) = (dir, dir);
    p.facing = super::yaw(dir);
    p.vel = Vec2::ZERO;
    super::set_motion(p, 0x2b, 1.0, false, None);
    g.whooshes.push((0, i, hst_sim::sound::hit_cry(i)));
}

/// The hit player stands in its motion from the hit to the point's reaction (in a match; the game's mode 3).
pub(super) fn standing(g: &Game, i: usize) -> bool {
    g.body_hit == Some(i as i32) && g.players.len() >= 2 && std::env::var("HST_BODY_HIT").is_err()
}

/// The pop-up is made the tick the hit is seen and ticks from then; it goes when the players react, and a new
/// point clears it.
fn age(g: Res<Game>, mut hits: ResMut<Hits>) {
    if g.phase == Phase::Serve || g.post.as_ref().is_some_and(|p| p.reacted) {
        hits.pop = None;
        hits.made &= g.phase != Phase::Serve;
        return;
    }
    if g.body_hit.is_some() && !hits.made {
        hits.made = true;
        hits.pop = Some(Popup::new(g.flight.ball.pos, hst_sim::sound::kmh(g.flight.ball.vel)));
    }
    let practice = g.players.len() < 2;
    hits.alpha = hits.pop.map_or(0.0, |p| p.alpha);
    if hits.pop.as_mut().is_some_and(|p| !p.tick(practice)) {
        hits.pop = None;
    }
}

fn draw(
    g: Res<Game>,
    time: Res<Time<Fixed>>,
    hits: Res<Hits>,
    cam: Query<&Transform, (With<crate::Orbit>, Without<View>)>,
    mut q: Query<(&View, &mut Transform, &mut Visibility)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(cam) = cam.single() else { return };
    for (v, mut tr, mut vis) in &mut q {
        let Some(p) = hits.pop else {
            *vis = Visibility::Hidden;
            continue;
        };
        // game space ↔ world: (x, -y, -z)
        let game = |w: Vec3| [w.x, -w.y, -w.z];
        let world = |p: [f32; 3]| Vec3::new(p[0], -p[1], -p[2]);
        let rows = [game(*cam.right()), game(-*cam.up()), game(*cam.forward())];
        let depth = |at: [f32; 3]| (world(at) - cam.translation).dot(*cam.forward());
        let (at, w, h) = p.quad(rows, depth, g.cam.view.fov.tan());
        tr.translation = world(at);
        tr.rotation = cam.rotation;
        tr.scale = Vec3::new(2.0 * w, 2.0 * h, 1.0);
        if let Some(mut m) = materials.get_mut(&v.0) {
            m.base_color_texture = Some(v.1[(p.word == Word::Smack) as usize].clone());
            // between the last two ticks' alphas (a fresh pop-up starts at its own)
            let a = if hits.alpha == 0.0 { p.alpha } else { hits.alpha + (p.alpha - hits.alpha) * time.overstep_fraction() };
            m.base_color = Color::srgba(1.0, 1.0, 1.0, a / 128.0);
        }
        *vis = Visibility::Visible;
    }
}

/// The point's bounce words (`hst_sim::bodyhit::BounceWords`), made from the serve's hit until the next point's
/// setup, and their textures by effect kind − 1.
#[derive(Resource)]
struct Words {
    words: BounceWords,
    on: bool,
    textures: Vec<Handle<Image>>,
}

#[derive(Component)]
struct WordView(usize, Handle<StandardMaterial>);

fn bounce_words(g: Res<Game>, time: Res<Time>, mut w: ResMut<Words>) {
    if g.phase == Phase::Serve {
        (w.words, w.on) = (BounceWords::default(), false);
        return;
    }
    w.on |= g.phase == Phase::Rally;
    if w.on {
        let (at, m) = g.flight.landing;
        let n = w.words.pops.len();
        w.words.frame(g.flight.bounces, at, &m);
        if w.words.pops.len() > n {
            info!("bounce word {} at {at:?}, {:.3} s in", w.words.pops[n].effect, time.elapsed_secs());
        }
    }
}

fn draw_words(
    g: Res<Game>,
    w: Res<Words>,
    cam: Query<&Transform, (With<crate::Orbit>, Without<WordView>)>,
    mut q: Query<(&WordView, &mut Transform, &mut Visibility)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(cam) = cam.single() else { return };
    let game = |w: Vec3| [w.x, -w.y, -w.z];
    let world = |p: [f32; 3]| Vec3::new(p[0], -p[1], -p[2]);
    let rows = [game(*cam.right()), game(-*cam.up()), game(*cam.forward())];
    let depth = |at: [f32; 3]| (world(at) - cam.translation).dot(*cam.forward());
    for (v, mut tr, mut vis) in &mut q {
        let Some(p) = w.words.pops.get(v.0) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let (at, hw, hh) = p.quad(rows, depth, g.cam.view.fov.tan());
        (tr.translation, tr.rotation, tr.scale) = (world(at), cam.rotation, Vec3::new(2.0 * hw, 2.0 * hh, 1.0));
        if let Some(mut m) = materials.get_mut(&v.1) {
            m.base_color_texture = Some(w.textures[p.effect as usize - 1].clone());
            m.base_color = Color::srgba(1.0, 1.0, 1.0, p.alpha / 128.0);
        }
        *vis = Visibility::Visible;
    }
}
