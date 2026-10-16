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
        .add_systems(FixedUpdate, tick.after(super::bodyhit::pose))
        .add_systems(Update, (draw.after(super::camera), draw_dash, draw_bits.after(super::camera)));
}

#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct FootFx {
    table: Foot,
    feet: Feet,
    court: usize,
    /// The court's dust colour (linear, `lin`).
    dust: [f32; 3],
    /// Last tick was in the serve, to clear everything as a point starts.
    serving: bool,
    wet: bool,
    #[serde(skip)]
    dry_puffs: Handle<Mesh>,
    #[serde(skip)]
    wet_puffs: Handle<Mesh>,
    #[serde(skip)]
    prints: Handle<Mesh>,
    /// One `run/dash` per player.
    dash: Vec<(Effect, Shown)>,
    /// Each player's dive as the run object sees it.
    dives: [DiveWatch; 4],
    /// Debris: clods, blades.
    #[serde(skip)]
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
    let dust = looks.get(court).unwrap_or(&looks[10]).dust.map(lin);
    let mut mesh = |tex: &str| {
        let m = meshes.add(Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default()));
        let material = look(&mut iso, tex, &mut materials, &mut images).expect("footstep texture");
        let view = commands.spawn((Mesh3d(m.clone()), MeshMaterial3d(material), Transform::default(), crate::cull::Cullable)).id();
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

/// A game colour channel (0..128, the GS's modulate in the framebuffer's gamma) as the linear vertex colour Bevy
/// multiplies the sRGB-decoded texture by; passed raw (`v / 128`) every colour showed brighter, footprints at court
/// 10's 25/128 as 48% grey (B286).
fn lin(v: f32) -> f32 {
    Color::srgb(v / 128.0, 0.0, 0.0).to_linear().red
}

/// One game frame: the players' toes and bones as the motion player posed them this tick, and their matrices,
/// step the sim. The run object updates after the players (it takes their hit events the same frame), so the dive
/// rings draw from the sound manager's generator after the frame's strokes.
fn tick(fx: Option<ResMut<FootFx>>, mut g: ResMut<Game>, w: Option<Res<Weather>>, posed: Res<super::bodyhit::Posed>, bones: Res<super::bodyhit::Bones>, q: Query<(&Figure, &Motion)>, rc: Option<Res<super::replay_camera::ReplayCamera>>) {
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
            posed.locals(i)?;
            // a skeleton without one of the bones has no runner
            let at = bones.of(i, data).at;
            at[0].and(at[3]).and(at[4]).and(at[5]).and(at[6])?;
            Some((
                i,
                Runner {
                    character: g.chars[i] as usize,
                    motion: m.id as i32,
                    sub: sub(&g, i, reacted),
                    m: super::player_matrix(p),
                    slide: states.get(i).is_some_and(|&s| s != 0),
                    dive,
                    lunge,
                    dive_over,
                    ..Runner::default()
                },
            ))
        })
        .collect();
    runners.sort_by_key(|(i, _)| *i);
    // the toes, pelvis, spine and head only when this frame's step can read them (`Feet::reads`): each is a chain of
    // VU0 products up the skeleton
    let runners: Vec<Runner> = runners
        .into_iter()
        .enumerate()
        .map(|(p, (i, mut r))| {
            let data = &g.data[i];
            let at = bones.of(i, data).at;
            let joint = |n: usize| at[n].zip(posed.locals(i)).map_or([[0.0; 4]; 4], |(k, locals)| hst_sim::pose::node_world(&data.skeleton, locals, k, &r.m));
            let [toes, pelvis, spine, head] = fx.feet.reads(&fx.table, p, r.character, r.motion, r.dive);
            if toes {
                r.toes = [joint(5)[3], joint(6)[3]];
            }
            if pelvis {
                r.pelvis = joint(4);
            }
            if spine {
                r.spine = joint(3);
            }
            if head {
                r.head = joint(0)[3];
            }
            r
        })
        .collect();
    let c = fx.table.courts.get(fx.court).map_or(10, |_| fx.court);
    let dusty = fx.table.courts[c].dusty && !fx.wet;
    let wind = w.map_or([0.0; 4], |w| hst_sim::weather::wind(w.today().degrees, w.today().speed));
    fx.feet.replay = g.instant.replaying;
    let mt = &mut g.rng.sound;
    fx.feet.tick(&fx.table, c, dusty, fx.wet, wind, &runners, true, &mut || mt.next());
    if g.instant.replaying && g.players.len() < 3 && rc.is_some_and(|rc| rc.close_up()) {
        steps(&mut g, &fx.feet, &states);
    }
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

/// A singles replay's close-up hears each footfall of a running player in front of the camera (`sound::step_heard`
/// at their origin), played there.
fn steps(g: &mut Game, feet: &Feet, states: &[u8; 4]) {
    let v = g.cam.view;
    let [x, y, z] = v.eye;
    let eye = [x, y, z, 1.0];
    let [x, y, z] = v.rot[2];
    let look = [x, y, z, 0.0];
    let view = super::reaction_voices::matrix(&v);
    let focal = 240.0 / hst_sim::musl::tanf(v.fov);
    for i in 0..g.players.len() {
        let p = &g.players[i];
        let at = [p.pos[0], p.pos[1], p.pos[2], 1.0];
        for _ in feet.stepped[i].iter().filter(|&&s| s && states[i] == 1 && hst_sim::sound::step_heard(eye, look, &view, focal, at)) {
            debug!("replay footstep, player {i}");
            g.whooshes.push((0, i, hst_sim::sound::step(crate::weather::now())));
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
/// `size` either side along row 0 and twice that up along row 1 of the basis the run object's draw builds facing
/// back along the camera's look (`impact_matrix(-look)`, level right); footprints lie on the court.
fn draw(fx: Option<Res<FootFx>>, g: Res<Game>, own: Option<Res<super::own_camera::OwnCam>>, mut meshes: ResMut<Assets<Mesh>>) {
    let Some(fx) = fx else { return };
    let [ax, ay, az] = super::own_camera::shown(&g, own.as_deref()).1.rot[2];
    let basis = hst_sim::effect::impact_matrix([-ax, -ay, -az, -0.0], [0.0; 3]);
    let v3 = |r: [f32; 4]| Vec3::new(r[0], r[1], r[2]);
    let mut put = |h: &Handle<Mesh>, quads: Vec<([Vec3; 4], [f32; 4])>| {
        let (mut pos, mut uv, mut colour, mut index) = (vec![], vec![], vec![], vec![]);
        for (corners, c) in quads {
            let n = pos.len() as u32;
            pos.extend(corners.map(|v| v.to_array()));
            uv.extend([[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0f32]]);
            colour.extend([c; 4]);
            index.extend([n, n + 1, n + 2, n + 2, n + 1, n + 3]);
        }
        fill(&mut meshes, h, pos, uv, colour, index);
    };
    let [r, g, b] = if fx.wet { [1.0; 3] } else { fx.dust };
    let puffs = fx
        .feet
        .puffs
        .iter()
        .map(|p| {
            use hst_sim::ps2::{add, madd, mul, sub};
            // pos + 0 − row 2 × 0.5 (the halving is exact, so one madd rounds as the game's mul and sub)
            let pull = |a: f32, r: f32| madd(add(a, 0.0), r, -0.5);
            let foot = [pull(p.pos[0], ax), pull(p.pos[1], ay), pull(p.pos[2], az)];
            let corner = |side: f32, top: bool| {
                Vec3::from_array(std::array::from_fn(|k| {
                    let at = add(foot[k], side * mul(basis[0][k], p.size));
                    if top { sub(at, mul(mul(basis[1][k], p.size), 2.0)) } else { at }
                }))
            };
            ([corner(-1.0, true), corner(1.0, true), corner(-1.0, false), corner(1.0, false)], [r, g, b, p.alpha / 128.0])
        })
        .collect();
    let (shown, empty) = if fx.wet { (&fx.wet_puffs, &fx.dry_puffs) } else { (&fx.dry_puffs, &fx.wet_puffs) };
    put(shown, puffs);
    put(empty, vec![]);
    let c = fx.table.courts.get(fx.court).unwrap_or(&fx.table.courts[10]);
    let [r, g, b] = c.print_rgb[fx.wet as usize].map(lin);
    let prints = fx
        .feet
        .prints
        .iter()
        .map(|p| {
            let (at, s, f) = (v3(p.m[3]), v3(p.m[0]) * 0.1, v3(p.m[2]) * 0.2);
            ([at - s - f, at + s - f, at - s + f, at + s + f], [r, g, b, p.alpha / 128.0])
        })
        .collect();
    put(&fx.prints, prints);
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
            let [r, g, b] = s.rgb.map(lin);
            quad((v3(s.m[0]) * s.size, v3(s.m[2]) * s.size), at, [r, g, b, 1.0]);
        } else {
            let [r, g, b] = c.clod_rgb.map(lin);
            quad((right * s.size, up * s.size), at, [r, g, b, 1.0]);
            quad((Vec3::X * s.size, Vec3::Z * s.size), Vec3::new(at.x, -0.01, at.z), [lin(32.0), lin(32.0), lin(32.0), 90.0 / 128.0]);
        }
    }
    let [shown, empty] = if blades { [&fx.bits[1], &fx.bits[0]] } else { [&fx.bits[0], &fx.bits[1]] };
    fill(&mut meshes, shown, pos, uv, colour, index);
    fill(&mut meshes, empty, vec![], vec![], vec![], vec![]);
}

// a resync's state (`snapshot::encode`) keeps this machine's drawing entities and handles
impl super::snapshot::Wire for FootFx {
    fn local(&mut self, l: &Self) {
        (self.dry_puffs, self.wet_puffs, self.prints, self.bits) = (l.dry_puffs.clone(), l.wet_puffs.clone(), l.prints.clone(), l.bits.clone());
        self.dash.iter_mut().zip(&l.dash).for_each(|(d, l)| d.1 = l.1.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::{Args, GameSpace};

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    /// A singles bot match's replay sounds footfalls only on its close-up closing shot, and does sound some there.
    #[test]
    fn replay_steps_sound_on_close_ups_only() {
        if !std::path::Path::new(ISO).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.init_asset::<crate::gs::GsMaterial>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 1, pads: None });
        app.insert_resource(Pads { seats: Some([false; 4]), autoplay: true, ..default() });
        app.init_resource::<tick_input::TickPads>();
        app.init_resource::<instant_replay::Recorder>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, bodyhit::plugin, super::plugin));
        app.add_systems(PostStartup, setup);
        app.add_systems(FixedUpdate, (remember, tick_input::systems(), control, simulate, motions, character::tick, played_out, held_ball, instant_replay::run).chain());
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(Startup);
        app.world_mut().run_schedule(PostStartup);
        // every closing shot a close-up of the hitter, so the first replay (the match point's) has one
        app.world_mut().resource_mut::<super::super::replay_camera::ReplayCamera>().picks.lists.end = [[0x58; 3]; 2];
        let step = hst_sim::sound::step(crate::weather::now());
        let (mut close, mut steps, mut replays) = (0, 0, 0);
        for _ in 0..60 * 60 * 120 {
            app.world_mut().run_schedule(FixedUpdate);
            let up = app.world().resource::<super::super::replay_camera::ReplayCamera>().close_up();
            let mut g = app.world_mut().resource_mut::<Game>();
            let n = g.whooshes.iter().filter(|w| w.2 == step).count();
            g.whooshes.clear();
            assert!(n == 0 || up, "a footstep off the close-ups");
            replays += g.instant.replaying as u32;
            close += up as u32;
            steps += n;
            if steps > 0 && !g.instant.replaying {
                break;
            }
        }
        eprintln!("{replays} replay ticks, {close} on close-ups, {steps} footsteps");
        assert!(close > 0 && steps > 0, "no footstep on a close-up");
    }
}
