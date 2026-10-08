//! Footstep puffs and footprints (`hst_sim::foot`): each player's toes (`Bip01RToe0`, `Bip01LToe0`) in game space
//! feed the sim every tick; puffs are drawn as camera-facing quads (`run/kemuri_00` dust in the court's dust colour,
//! `run/spray` grey in rain), footprints as ground quads (`run/e_footprint` in the court's print colour); and each
//! player's `run/dash` streak model at the hips through a dive.

use bevy::mesh::morph::MorphWeights;
use bevy::prelude::*;
use hst_data::{exe::Foot, iso::Iso, xb::Archive};
use hst_sim::effect::Effect;
use hst_sim::foot::{Feet, Runner};
use hst_sim::weather::Mt;

use super::{Figure, Game, Phase};
use crate::character::{Motion, Rig};
use crate::effects::{Shown, fill, look, model, pose};
use crate::weather::Weather;
use crate::Args;

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, setup.after(super::setup))
        .add_systems(FixedUpdate, tick.after(crate::character::tick))
        .add_systems(Update, (draw.after(super::camera), draw_dash));
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
    /// A dive was under way last tick, per player.
    diving: [bool; 4],
    /// The dive rings' random draws.
    // ponytail: its own generator, not the match's shared one
    mt: Mt,
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
    let arc = iso.read("AZUMA/C_EFF/EFFCT.XB0").expect("EFFCT.XB0");
    let arc = Archive::parse(&arc).expect("EFFCT.XB0");
    let dash = (0..4).map(|_| model(&arc, "run/dash", &mut commands, root, &mut meshes, &mut materials, &mut images, &mut bindposes).expect("run/dash")).collect();
    let (table, feet, mt) = (game.foot(), Feet::default(), Mt::new(1));
    commands.insert_resource(FootFx { table, feet, court, dust, serving: false, wet: false, dry_puffs, wet_puffs, prints, dash, diving: [false; 4], mt });
}

/// One game frame: the players' toes and matrices (game space, from the last drawn pose) step the sim.
// ponytail: the toes are the last drawn pose (one frame behind the motion), as the swing trails
fn tick(
    fx: Option<ResMut<FootFx>>,
    g: Res<Game>,
    w: Option<Res<Weather>>,
    q: Query<(&Figure, &Rig, &Motion, &GlobalTransform)>,
    joints: Query<&GlobalTransform>,
    root: Query<&GlobalTransform, With<crate::GameSpace>>,
) {
    let (Some(mut fx), Ok(root)) = (fx, root.single()) else { return };
    let fx = &mut *fx;
    let to_game = root.affine().inverse();
    // the game clears the run object as a point is set up
    if g.phase == Phase::Serve && !fx.serving {
        fx.feet = Feet::default();
    }
    fx.serving = g.phase == Phase::Serve;
    fx.feet.after_point = g.phase == Phase::Post;
    fx.wet = w.is_some_and(|w| hst_sim::weather::rain(w.today().weather));
    let mut runners: Vec<(usize, Runner)> = q
        .iter()
        .filter_map(|(f, rig, m, gt)| {
            let joint = |n: &str| rig.data.joint(n).and_then(|j| joints.get(rig.joints[j]).ok()).map(|t| Mat4::from(to_game * t.affine()));
            let toe = |n: &str| joint(n).map(|m| m.w_axis.to_array());
            let cols = |m: Mat4| [m.x_axis, m.y_axis, m.z_axis, m.w_axis].map(|c| c.to_array());
            let pm = Mat4::from(to_game * gt.affine());
            let dive = g.players.get(f.0).and_then(|p| p.dive.as_ref());
            Some((
                f.0,
                Runner {
                    character: g.chars[f.0] as usize,
                    motion: m.id as i32,
                    // ponytail: the motion's sub-state (it only stops steps late in a high motion past the point) is 0
                    sub: 0,
                    toes: [toe("Bip01RToe0")?, toe("Bip01LToe0")?],
                    m: [pm.x_axis, pm.y_axis, pm.z_axis, pm.w_axis].map(|c| c.to_array()),
                    // ponytail: the player's state byte (1 running, 2 swinging, 0 else) from the motion id
                    slide: (3..=0x1f).contains(&m.id),
                    pelvis: cols(joint("Bip01Pelvis")?),
                    spine: cols(joint("Bip01Spine1")?),
                    head: toe("Bip01Head")?,
                    // a dive starting: the game raises it with the hit event's dive branch
                    dive: dive.is_some() && !fx.diving.get(f.0).copied().unwrap_or(true),
                    lunge: dive.map_or(0.0, |d| d.slide),
                    // ponytail: the game's own end-of-dive byte taken as the dive being over
                    dive_over: dive.is_none(),
                    ..Runner::default()
                },
            ))
        })
        .collect();
    runners.sort_by_key(|(i, _)| *i);
    for (i, d) in fx.diving.iter_mut().enumerate() {
        *d = g.players.get(i).is_some_and(|p| p.dive.is_some());
    }
    let runners: Vec<Runner> = runners.into_iter().map(|(_, r)| r).collect();
    let c = fx.table.courts.get(fx.court).map_or(10, |_| fx.court);
    let dusty = fx.table.courts[c].dusty && !fx.wet;
    // ponytail: no wind drift (the weather's wind vector is not ported to the scene)
    let mt = &mut fx.mt;
    fx.feet.tick(&fx.table, c, dusty, fx.wet, [0.0; 4], &runners, true, &mut || mt.next());
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

/// Puffs face the camera, pushed 0.5 toward it, `size` either side and twice that tall; footprints lie on the court.
fn draw(fx: Option<Res<FootFx>>, cam: Query<&Transform, With<crate::Orbit>>, mut meshes: ResMut<Assets<Mesh>>) {
    let (Some(fx), Ok(cam)) = (fx, cam.single()) else { return };
    let game = |v: Vec3| Vec3::new(v.x, -v.y, -v.z);
    let (right, up, ahead) = (game(cam.rotation * Vec3::X), game(cam.rotation * Vec3::Y), game(cam.rotation * Vec3::NEG_Z));
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
    // ponytail: the game pulls the puff by its view matrix's third row × 0.5; taken as 0.5 along the view toward the camera
    let [r, g, b] = if fx.wet { [1.0; 3] } else { fx.dust };
    let puffs = fx
        .feet
        .puffs
        .iter()
        .map(|p| {
            let (foot, w, h) = (v3(p.pos) - ahead * 0.5, right * p.size, up * 2.0 * p.size);
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
