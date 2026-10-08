//! Footstep puffs and footprints (`hst_sim::foot`): each player's toes (`Bip01RToe0`, `Bip01LToe0`) in game space
//! feed the sim every tick; puffs are drawn as camera-facing quads (`run/kemuri_00` dust in the court's dust colour,
//! `run/spray` grey in rain), footprints as ground quads (`run/e_footprint` in the court's print colour); and each
//! player's `run/dash` streak model at the hips through a dive. An instant replay's dive debris: clods
//! (`par/tuti`, camera-facing, with a ground shadow) or grass blades (`par/siba`, lying in their turn).

use bevy::mesh::morph::MorphWeights;
use bevy::prelude::*;
use hst_data::{exe::Foot, iso::Iso, xb::Archive};
use hst_sim::effect::Effect;
use hst_sim::foot::{DiveWatch, Feet, Runner};

use super::{Figure, Game, Phase};
use crate::character::Motion;
use crate::effects::{Shown, fill, look, model, pose};
use crate::weather::Weather;
use crate::Args;

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, setup.after(super::setup))
        .add_systems(FixedUpdate, tick.after(crate::character::tick))
        .add_systems(Update, (draw.after(super::camera), draw_dash, draw_bits.after(super::camera)));
}

#[derive(Resource)]
struct FootFx {
    table: Foot,
    feet: Feet,
    court: usize,
    /// The court's dust colour (1 = the game's 128).
    dust: [f32; 3],
    /// Last tick was in the serve, to clear everything as a point starts.
    serving: bool,
    wet: bool,
    dry_puffs: Handle<Mesh>,
    wet_puffs: Handle<Mesh>,
    prints: Handle<Mesh>,
    /// One `run/dash` per player.
    dash: Vec<(Effect, Shown)>,
    /// Each player's dive as the run object sees it.
    dives: [DiveWatch; 4],
    /// Debris: clods, blades.
    bits: [Handle<Mesh>; 2],
    /// Each player's state byte (`state`), held once the point is over.
    states: [u8; 4],
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    root: Query<Entity, With<crate::GameSpace>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut bindposes: ResMut<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>,
) {
    let Ok(root) = root.single() else { return };
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"));
    let game = hst_data::exe::Game::new(&cnf, &bin).expect("game program");
    let court = args.stage.map_or(args.court, |s| s as usize);
    let looks = game.bounce_looks();
    let dust = looks.get(court).unwrap_or(&looks[10]).dust.map(|v| v / 128.0);
    let mut mesh = |tex: &str| {
        let m = meshes.add(Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default()));
        let material = look(&mut iso, tex, &mut materials, &mut images).expect("footstep texture");
        let view = commands.spawn((Mesh3d(m.clone()), MeshMaterial3d(material), Transform::default(), bevy::camera::visibility::NoFrustumCulling)).id();
        commands.entity(root).add_child(view);
        m
    };
    let (dry_puffs, wet_puffs, prints) = (mesh("run/kemuri_00"), mesh("run/spray"), mesh("run/e_footprint"));
    let bits = [mesh("par/tuti"), mesh("par/siba")];
    let arc = iso.read("AZUMA/C_EFF/EFFCT.XB0").expect("EFFCT.XB0");
    let arc = Archive::parse(&arc).expect("EFFCT.XB0");
    let dash = (0..4).map(|_| model(&arc, "run/dash", &mut commands, root, &mut meshes, &mut materials, &mut images, &mut bindposes).expect("run/dash")).collect();
    let (table, feet) = (game.foot(), Feet::default());
    commands.insert_resource(FootFx { table, feet, court, dust, serving: false, wet: false, dry_puffs, wet_puffs, prints, dash, dives: [DiveWatch::default(); 4], bits, states: [0; 4] });
}

/// One game frame: the players' toes and bones as the motion player posed them this tick, and their matrices,
/// step the sim. The run object updates after the players (it takes their hit events the same frame), so the dive
/// rings draw from the sound manager's generator after the frame's strokes.
fn tick(fx: Option<ResMut<FootFx>>, mut g: ResMut<Game>, w: Option<Res<Weather>>, q: Query<(&Figure, &Motion)>) {
    let Some(mut fx) = fx else { return };
    let fx = &mut *fx;
    // the game clears the run object as a point is set up
    if g.phase == Phase::Serve && !fx.serving {
        fx.feet = Feet::default();
    }
    fx.serving = g.phase == Phase::Serve;
    fx.feet.after_point = g.phase == Phase::Post;
    fx.wet = w.as_ref().is_some_and(|w| hst_sim::weather::rain(w.today().weather));
    let reacted = g.post.as_ref().is_some_and(|p| p.reacted);
    for i in (0..g.players.len().min(4)).filter(|_| !fx.feet.after_point) {
        fx.states[i] = state(&g, i);
    }
    let states = fx.states;
    let mut runners: Vec<(usize, Runner)> = q
        .iter()
        .filter_map(|(f, m)| {
            let (i, data) = (f.0, g.data.get(f.0)?);
            let p = g.players.get(i)?;
            let pd = p.dive.as_ref();
            let (dive, lunge, dive_over) = fx.dives.get_mut(i).map_or((false, 0.0, false), |w| w.see(pd.map(|d| (d.slide, d.cut()))));
            let sk = &data.skeleton;
            let locals = super::bodyhit::posed(data, m)?;
            let pm = super::player_matrix(p);
            let joint = |n: &str| sk.names.iter().position(|b| b == n).map(|k| hst_sim::pose::node_world(sk, &locals, k, &pm));
            Some((
                i,
                Runner {
                    character: g.chars[i] as usize,
                    motion: m.id as i32,
                    sub: sub(&g, i, reacted),
                    toes: [joint("Bip01RToe0")?[3], joint("Bip01LToe0")?[3]],
                    m: pm,
                    slide: states.get(i).is_some_and(|&s| s != 0),
                    pelvis: joint("Bip01Pelvis")?,
                    spine: joint("Bip01Spine1")?,
                    head: joint("Bip01Head")?[3],
                    dive,
                    lunge,
                    dive_over,
                    // ponytail: where the motion set off, taken as a step back along the dive (the player hasn't moved yet)
                    anchor: pd.map_or([0.0; 4], |d| [pm[3][0] - d.dir[0], pm[3][1], pm[3][2] - d.dir[1], pm[3][3]]),
                    ..Runner::default()
                },
            ))
        })
        .collect();
    runners.sort_by_key(|(i, _)| *i);
    let runners: Vec<Runner> = runners.into_iter().map(|(_, r)| r).collect();
    let c = fx.table.courts.get(fx.court).map_or(10, |_| fx.court);
    let dusty = fx.table.courts[c].dusty && !fx.wet;
    let wind = w.map_or([0.0; 4], |w| hst_sim::weather::wind(w.today().degrees, w.today().speed));
    // ponytail: no instant replay yet (P0b4d), so dives throw no debris
    fx.feet.replay = false;
    let mt = &mut g.rng.sound;
    fx.feet.tick(&fx.table, c, dusty, fx.wet, wind, &runners, true, &mut || mt.next());
    // the streak plays from each dive's start while it shows
    for (p, (e, _)) in fx.dash.iter_mut().enumerate() {
        if fx.feet.dash_start[p] {
            e.start();
        }
        if fx.feet.dash[p].is_some() {
            e.tick();
        }
    }
}

/// The player's state byte as the game's mode setter keeps it: 3 standing hit by the ball, 2 in a stroke (a press
/// still searching, a locked swing through its follow-through, a whiff or a dive), 1 running (the stick held), else
/// 0. It holds from the point's end to the next point (`tick` keeps it; foot_s05 has no change past the point).
fn state(g: &Game, i: usize) -> u8 {
    let p = &g.players[i];
    if super::bodyhit::standing(g, i) {
        3
    } else if p.contact.is_some() || p.pending.is_some() || p.wait_swing.is_some() || p.swing.is_some() || p.whiff.is_some() || p.dive.is_some() {
        2
    } else {
        p.body.running as u8
    }
}

/// The player's reaction motion as the game files it at the players' reaction (0x2b for the player the ball hit;
/// the point's setup clears it), else 0.
fn sub(g: &Game, i: usize, reacted: bool) -> i32 {
    match () {
        _ if !reacted => 0,
        _ if super::bodyhit::standing(g, i) => 0x2b,
        _ => g.players[i].root.map_or(0, |r| r.motion as i32),
    }
}

/// Puffs face the camera, pulled back half a metre along the game view's forward row (its camera → world row 2),
/// `size` either side and twice that tall; footprints lie on the court.
fn draw(fx: Option<Res<FootFx>>, g: Res<Game>, cam: Query<&Transform, With<crate::Orbit>>, mut meshes: ResMut<Assets<Mesh>>) {
    let (Some(fx), Ok(cam)) = (fx, cam.single()) else { return };
    let game = |v: Vec3| Vec3::new(v.x, -v.y, -v.z);
    let (right, up) = (game(cam.rotation * Vec3::X), game(cam.rotation * Vec3::Y));
    let [ax, ay, az] = g.cam.view.rot[2];
    let v3 = |r: [f32; 4]| Vec3::new(r[0], r[1], r[2]);
    let put = |mesh: &mut Mesh, quads: Vec<([Vec3; 4], [f32; 4])>| {
        let (mut pos, mut uv, mut colour, mut index) = (vec![], vec![], vec![], vec![]);
        for (corners, c) in quads {
            let n = pos.len() as u32;
            pos.extend(corners.map(|v| v.to_array()));
            uv.extend([[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0f32]]);
            colour.extend([c; 4]);
            index.extend([n, n + 1, n + 2, n + 2, n + 1, n + 3]);
        }
        fill(mesh, pos, uv, colour, index);
    };
    let [r, g, b] = if fx.wet { [1.0; 3] } else { fx.dust };
    let puffs = fx
        .feet
        .puffs
        .iter()
        .map(|p| {
            use hst_sim::ps2::{add, madd};
            // pos + 0 − row 2 × 0.5 (the halving is exact, so one madd rounds as the game's mul and sub)
            let pull = |a: f32, r: f32| madd(add(a, 0.0), r, -0.5);
            let foot = Vec3::new(pull(p.pos[0], ax), pull(p.pos[1], ay), pull(p.pos[2], az));
            let (w, h) = (right * p.size, up * 2.0 * p.size);
            ([foot - w + h, foot + w + h, foot - w, foot + w], [r, g, b, p.alpha / 128.0])
        })
        .collect();
    let (shown, empty) = if fx.wet { (&fx.wet_puffs, &fx.dry_puffs) } else { (&fx.dry_puffs, &fx.wet_puffs) };
    if let Some(mut mesh) = meshes.get_mut(shown) {
        put(&mut mesh, puffs);
    }
    if let Some(mut mesh) = meshes.get_mut(empty) {
        put(&mut mesh, vec![]);
    }
    let c = fx.table.courts.get(fx.court).unwrap_or(&fx.table.courts[10]);
    let [r, g, b] = c.print_rgb[fx.wet as usize].map(|v| v / 128.0);
    let prints = fx
        .feet
        .prints
        .iter()
        .map(|p| {
            let (at, s, f) = (v3(p.m[3]), v3(p.m[0]) * 0.1, v3(p.m[2]) * 0.2);
            ([at - s - f, at + s - f, at - s + f, at + s + f], [r, g, b, p.alpha / 128.0])
        })
        .collect();
    if let Some(mut mesh) = meshes.get_mut(&fx.prints) {
        put(&mut mesh, prints);
    }
}

/// Each player's `run/dash` streak at the yaw and position of the hips as its dive started.
fn draw_dash(fx: Option<Res<FootFx>>, mut q: Query<(&mut Visibility, &mut MorphWeights)>, mut joints: Query<&mut Transform>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let Some(fx) = fx else { return };
    for ((effect, view), at) in fx.dash.iter().zip(fx.feet.dash) {
        match at {
            Some(m) if effect.live => {
                pose(effect, view, &mut q, &mut joints, &mut materials);
                if let Ok(mut t) = joints.get_mut(view.root) {
                    *t = Transform::from_matrix(Mat4::from_cols_array_2d(&m));
                }
            }
            _ => {
                if let Ok((mut v, _)) = q.get_mut(view.root) {
                    *v = Visibility::Hidden;
                }
            }
        }
    }
}

/// Debris, all in group 0's look: a clod faces the camera (`size` each way, the court's clod colour) over a dark
/// shadow on the court; a blade lies in its turn's rows 0 and 2 in its own colour. Each shows one 32-pixel cell.
fn draw_bits(fx: Option<Res<FootFx>>, cam: Query<&Transform, With<crate::Orbit>>, mut meshes: ResMut<Assets<Mesh>>) {
    let (Some(fx), Ok(cam)) = (fx, cam.single()) else { return };
    let game = |v: Vec3| Vec3::new(v.x, -v.y, -v.z);
    let (right, up) = (game(cam.rotation * Vec3::X), game(cam.rotation * Vec3::Y));
    let v3 = |r: [f32; 4]| Vec3::new(r[0], r[1], r[2]);
    let f = &fx.feet;
    let blades = f.kinds[0] == 1;
    let c = fx.table.courts.get(fx.court).unwrap_or(&fx.table.courts[10]);
    let (mut pos, mut uv, mut colour, mut index) = (vec![], vec![], vec![], vec![]);
    for s in f.bits[..f.groups].iter().flatten().filter(|s| s.alive) {
        // ponytail: both sheets are 64 pixels square (the cells sit at 0 and 32)
        let [u0, v0] = s.cell.map(|c| (c as f32 + 0.5) / 64.0);
        let [u1, v1] = s.cell.map(|c| (c as f32 + 31.5) / 64.0);
        let at = v3(s.m[3]);
        let mut quad = |(a, b): (Vec3, Vec3), at: Vec3, c: [f32; 4]| {
            let n = pos.len() as u32;
            pos.extend([at - a + b, at + a + b, at - a - b, at + a - b].map(|v| v.to_array()));
            uv.extend([[u0, v0], [u1, v0], [u0, v1], [u1, v1]]);
            colour.extend([c; 4]);
            index.extend([n, n + 1, n + 2, n + 2, n + 1, n + 3]);
        };
        if blades {
            let [r, g, b] = s.rgb.map(|v| v / 128.0);
            quad((v3(s.m[0]) * s.size, v3(s.m[2]) * s.size), at, [r, g, b, 1.0]);
        } else {
            let [r, g, b] = c.clod_rgb.map(|v| v / 128.0);
            quad((right * s.size, up * s.size), at, [r, g, b, 1.0]);
            quad((Vec3::X * s.size, Vec3::Z * s.size), Vec3::new(at.x, -0.01, at.z), [0.25, 0.25, 0.25, 90.0 / 128.0]);
        }
    }
    let [shown, empty] = if blades { [&fx.bits[1], &fx.bits[0]] } else { [&fx.bits[0], &fx.bits[1]] };
    if let Some(mut mesh) = meshes.get_mut(shown) {
        fill(&mut mesh, pos, uv, colour, index);
    }
    if let Some(mut mesh) = meshes.get_mut(empty) {
        fill(&mut mesh, vec![], vec![], vec![], vec![]);
    }
}
