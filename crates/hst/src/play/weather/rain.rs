//! The weather's particles (`AZUMA/C_EFF/EFFCT.XB0`, `hatsuyama/efct/`), in game space (y down):
//! - rain (weathers 2, 3): 20 streaks (`rain00`) falling through a 0.9 × 0.6 × 0.2 box half a metre in front of
//!   the camera, and a 5 × 12 grid of 3 m ripple cells (`groundrain`, four frames) on the court;
//! - wind (3 or more, clear or cloudy): leaves (`leaf00..08`) blown across the courts that have them, landing,
//!   sliding and fading.
//!
//! Their random source is the game's effects LCG, restarted from the match's seed at every serve.
//! Rain also plays its ambience: court sound program 1 key 2 from four bearings (90, 135, 225, 270).
//! Each rain voice sweeps ±45° about its bearing, a degree every 5 ticks.
//! They're drawn in the court's fogged pass: GS materials, unlit, under the court's fog (`weather::apply`).

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::camera::visibility::NoFrustumCulling;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use hst_data::iso::Iso;

use super::super::{Game, Phase};
use crate::effects::{fill, look};
use crate::weather::Weather;
use crate::gs::{GsKey, GsMaterial, GsUniform, NO_FOG, Test};
use crate::{Args, Orbit};

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup).add_systems(FixedUpdate, (tick, ambience)).add_systems(Update, draw);
}

/// The game's effects random source: an LCG giving 15-bit draws in [0, 1).
#[derive(Clone, Copy)]
struct Lcg(u32);

impl Lcg {
    fn r(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(0x343fd).wrapping_add(0x269ec3);
        ((self.0 >> 16) & 0x7fff) as f32 * 3.051_757_8e-5
    }
    /// −1..1
    fn s(&mut self) -> f32 {
        2.0 * self.r() - 1.0
    }
}

fn wrap_pi(a: f32) -> f32 {
    (a + PI).rem_euclid(TAU) - PI
}

const STREAKS: usize = 20;
/// The streak box's half extents (camera right, down, forward) and its distance ahead.
const BOX: Vec3 = Vec3::new(0.45, 0.3, 0.1);
const AHEAD: f32 = 0.5;
/// The streaks are drawn this much further out (and as much bigger: the same on screen) to clear the port's 5 m
/// near plane.
const FAR: f32 = 15.0;

/// Ground rain: cells across and along, their size, and how many ticks each frame shows.
const CELLS: [usize; 2] = [5, 12];
const CELL: f32 = 3.0;
const FRAME: u32 = 5;

const SLOTS: usize = 20;
const LIFE: i32 = 400;
/// Each leaf type's quad half-size and base spin per tick.
const HALF: [f32; 9] = [0.06, 0.15, 0.15, 0.08, 0.2, 0.04, 0.04, 0.04, 0.25];
const SPIN: [f32; 9] = [0.2, 0.2, 0.2, 0.2, 0.06, 0.2, 0.2, 0.2, 0.2];
/// Each court's leaf kinds (type, strength), one per wind object.
const COURT_LEAVES: [&[(usize, i32)]; 14] = [
    &[],
    &[(0, 2)],
    &[(5, 2), (6, 2), (7, 2)],
    &[(8, 2)],
    &[(0, 2), (5, 3)],
    &[],
    &[(0, 2), (4, 2)],
    &[],
    &[(0, 2), (4, 3), (8, 2)],
    &[(0, 1), (0, 2), (8, 2)],
    &[(0, 2), (4, 2), (7, 2)],
    &[(1, 2), (2, 2), (3, 2)],
    &[],
    &[],
];

#[derive(Clone, Copy, Default)]
struct Leaf {
    live: bool,
    landed: bool,
    t: i32,
    pos: Vec3,
    shown: Vec3,
    phase: f32,
    rate: f32,
    spin: f32,
    spin_rate: f32,
    flip: bool,
    slide: Vec3,
    alpha: f32,
}

/// One wind object: a ring of leaves of one type.
#[derive(Clone, Copy, Default)]
struct Wind {
    kind: usize,
    strength: i32,
    on: bool,
    started: bool,
    tick: i32,
    interval: i32,
    next: usize,
    speed: i32,
    weather: u8,
    leaves: [Leaf; SLOTS],
}

fn blow(degrees: f32, speed: f32) -> (f32, Vec3) {
    let a = wrap_pi(degrees.to_radians());
    (a, Vec3::new(a.sin(), 0.0, a.cos()) * speed * 0.011)
}

impl Wind {
    /// Spawn ticks: 700 ± 50%, longer for stronger kinds, shorter in more wind.
    fn interval(&self, rng: &mut Lcg, speed: f32) -> i32 {
        let i = (700.0 * (1.0 - 0.5 * rng.r())) as i32;
        let i = (i as f32 * ((self.strength as f32 - 1.0) / 3.0 + 1.0)) as i32;
        (i as f32 * (1.0 - (speed - 2.0).max(0.0) * 0.6 / 7.0)) as i32
    }

    fn setup(&mut self, rng: &mut Lcg, speed: f32, weather: u8) {
        *self = Wind { kind: self.kind, strength: self.strength, speed: speed as i32, weather, ..default() };
        self.on = self.strength != 0 && speed as i32 >= 3 && weather < 2;
        self.interval = self.interval(rng, speed);
    }

    fn tick(&mut self, rng: &mut Lcg, degrees: f32, speed: f32, weather: u8) {
        if self.speed as f32 != speed || self.weather != weather {
            self.setup(rng, speed, weather);
        }
        if !self.on {
            return;
        }
        let (a, w) = blow(degrees, speed);
        let first = !self.started;
        if first {
            self.started = true;
            self.tick = (LIFE as f32 * (0.8 + rng.r() * 0.15)) as i32;
        }
        if speed >= 3.0 && (first || self.tick % self.interval.max(1) == 0) {
            self.spawn(rng, w, speed, first);
        }
        let across = Vec3::new(a.cos(), 0.0, -a.sin());
        for l in self.leaves.iter_mut().filter(|l| l.live) {
            if l.landed {
                l.alpha = (1.0 - l.t as f32 / 60.0) * 128.0;
                l.pos += l.slide;
                l.shown = l.pos;
                l.t += 1;
                l.live = l.t < 60;
            } else {
                if l.t <= 30 {
                    l.alpha = l.t as f32 / 30.0 * 128.0;
                }
                l.pos += w + Vec3::Y * 0.015;
                l.phase = wrap_pi(l.phase + l.rate);
                l.shown = l.pos + across * l.phase.sin() * 0.5;
                l.spin = wrap_pi(l.spin + l.spin_rate);
                l.t += 1;
                if l.t >= LIFE {
                    (l.landed, l.t) = (true, 0);
                }
            }
        }
        self.tick += 1;
    }

    /// A leaf timed to land at a random spot (±9, ±15) when its flight ends. The first of a point starts in
    /// mid-flight; its fade-in has passed, so it shows only once landed (as in the game).
    fn spawn(&mut self, rng: &mut Lcg, w: Vec3, speed: f32, first: bool) {
        let left = if first { LIFE - self.tick } else { LIFE } as f32;
        let x = 9.0 * rng.s();
        let z = 15.0 * rng.s();
        let sign = if rng.r() < 0.5 { -1.0 } else { 1.0 };
        let rate = 0.015 * (1.0 - 0.6 * rng.r()) * sign;
        let spin_rate = 0.01 * speed + SPIN[self.kind];
        let flip = rng.r() >= 0.5;
        self.leaves[self.next] = Leaf {
            live: true,
            t: if first { self.tick } else { 0 },
            pos: Vec3::new(x, -0.01, z) - w * left - Vec3::Y * 0.015 * left,
            phase: wrap_pi(-rate * LIFE as f32),
            rate,
            spin: wrap_pi(-spin_rate * LIFE as f32),
            spin_rate,
            flip,
            slide: Vec3::new(w.x, 0.0, w.z) * 0.6,
            ..default()
        };
        self.next = (self.next + 1) % SLOTS;
        self.interval = self.interval(rng, speed);
        if first {
            self.interval = (self.interval - self.tick).max(1);
        }
    }
}

/// Row-vector rotations (the game's matrix order: `a · b` applies `a` first).
type M3 = [Vec3; 3];
fn mul(a: M3, b: M3) -> M3 {
    a.map(|r| b[0] * r.x + b[1] * r.y + b[2] * r.z)
}
fn rot_x(t: f32) -> M3 {
    let (s, c) = t.sin_cos();
    [Vec3::X, Vec3::new(0.0, c, s), Vec3::new(0.0, -s, c)]
}
fn rot_y(t: f32) -> M3 {
    let (s, c) = t.sin_cos();
    [Vec3::new(c, 0.0, -s), Vec3::Y, Vec3::new(s, 0.0, c)]
}
fn rot_z(t: f32) -> M3 {
    let (s, c) = t.sin_cos();
    [Vec3::new(c, s, 0.0), Vec3::new(-s, c, 0.0), Vec3::Z]
}

#[derive(Resource)]
struct Particles {
    seed: u32,
    rng: Lcg,
    tick: u32,
    rain: bool,
    /// The streaks in world space and their velocity.
    streaks: [Vec3; STREAKS],
    fall: Vec3,
    camera: Option<Vec3>,
    /// The ground grid's offset.
    ground: Vec2,
    winds: Vec<Wind>,
    serving: bool,
    views: Vec<(Handle<Mesh>, Entity)>,
}

fn setup(mut commands: Commands, args: Res<Args>, mut meshes: ResMut<Assets<Mesh>>, mut standard: ResMut<Assets<StandardMaterial>>, mut materials: ResMut<Assets<GsMaterial>>, mut images: ResMut<Assets<Image>>) {
    let Some(court) = args.stage else { return };
    let Ok(mut iso) = Iso::open(&args.iso) else { return };
    let mut views = vec![];
    for name in ["rain00", "groundrain"].into_iter().map(String::from).chain((0..9).map(|k| format!("leaf{k:02}"))) {
        let Ok(m) = look(&mut iso, &format!("hatsuyama/efct/{name}"), &mut standard, &mut images) else { return };
        let texture = standard.remove(&m).and_then(|m| m.base_color_texture);
        if let Some(mut img) = texture.as_ref().and_then(|h| images.get_mut(h)) {
            img.texture_descriptor.format = bevy::render::render_resource::TextureFormat::Rgba8Unorm;
            if name == "rain00" {
                img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor { address_mode_u: ImageAddressMode::Repeat, ..ImageSamplerDescriptor::nearest() });
            }
        }
        // ponytail: blended without a Z write (`Test::Never`), as they were as standard materials (P17n)
        let uniform = GsUniform { color: Vec4::ONE, shininess: 1.0, highlight: 0.0, shadow: 0.0, uv_offset: Vec2::ZERO, fog: NO_FOG, fog_color: Vec4::ONE, lod_k: 0.0, light_dir: Vec4::ZERO, light_color: Vec4::ZERO, ambient: Vec4::ONE };
        let key = GsKey { textured: true, modulate: true, test: Test::Never, blend: Some(hst_data::mtl::Blend::Normal), fog: true, cull: false };
        let m = materials.add(GsMaterial { uniform, texture, key });
        let mesh = meshes.add(Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default()));
        let e = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(m), Transform::from_rotation(Quat::from_rotation_x(PI)), NoFrustumCulling, bevy::light::NotShadowCaster)).id();
        views.push((mesh, e));
    }
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos() | 1);
    let winds = COURT_LEAVES.get(court as usize).copied().unwrap_or_default().iter().map(|&(kind, strength)| Wind { kind, strength, speed: -1, ..default() }).collect();
    commands.insert_resource(Particles { seed, rng: Lcg(seed), tick: 0, rain: false, streaks: [Vec3::ZERO; STREAKS], fall: Vec3::ZERO, camera: None, ground: Vec2::ZERO, winds, serving: false, views });
}

/// The camera's basis in game space: right, down, forward, position.
fn basis(t: &Transform) -> [Vec3; 4] {
    let game = |v: Vec3| Vec3::new(v.x, -v.y, -v.z);
    [game(t.rotation * Vec3::X), game(t.rotation * Vec3::NEG_Y), game(t.rotation * Vec3::NEG_Z), game(t.translation)]
}

impl Particles {
    fn respawn(&mut self, cam: [Vec3; 4]) {
        for s in &mut self.streaks {
            let (z, y, x) = (BOX.z * self.rng.s(), BOX.y * self.rng.s(), BOX.x * self.rng.s());
            let _phase = self.rng.r();
            *s = cam[3] + cam[0] * x + cam[1] * y + cam[2] * (z + AHEAD);
        }
        self.camera = Some(cam[3]);
    }

    /// The streaks fall and drift downwind; the camera's motion streams them through the box, and one that leaves
    /// it comes back in on the opposite face.
    fn rain(&mut self, cam: [Vec3; 4], degrees: f32, speed: f32) {
        if self.camera.is_none_or(|c| c.distance(cam[3]) > 2.0 * AHEAD) {
            self.respawn(cam);
        }
        self.camera = Some(cam[3]);
        let a = wrap_pi(degrees.to_radians());
        self.fall = Vec3::Y * 0.0035 + Vec3::new(a.sin(), 0.0, a.cos()) * speed * 0.0005;
        for i in 0..STREAKS {
            let d = self.streaks[i] + self.fall - cam[3];
            let mut p = Vec3::new(d.dot(cam[0]), d.dot(cam[1]), d.dot(cam[2]) - AHEAD);
            if p.x.abs() >= BOX.x {
                p.x -= 2.0 * BOX.x * p.x.signum();
                (p.y, p.z) = (BOX.y * self.rng.s(), BOX.z * self.rng.s());
            } else if p.y.abs() >= BOX.y {
                p.y -= 2.0 * BOX.y * p.y.signum();
                (p.x, p.z) = (BOX.x * self.rng.s(), BOX.z * self.rng.s());
            } else if p.z.abs() >= BOX.z {
                p.z -= 2.0 * BOX.z * p.z.signum();
                (p.x, p.y) = (BOX.x * self.rng.s(), BOX.y * self.rng.s());
            }
            self.streaks[i] = cam[3] + cam[0] * p.x + cam[1] * p.y + cam[2] * (p.z + AHEAD);
        }
    }
}

fn tick(fx: Option<ResMut<Particles>>, g: Res<Game>, w: Option<Res<Weather>>, cam: Query<&Transform, With<Orbit>>) {
    let (Some(mut fx), Some(w), Ok(cam)) = (fx, w, cam.single()) else { return };
    let fx = &mut *fx;
    let today = w.today();
    let cam = basis(cam);
    let serving = g.phase == Phase::Serve;
    let serve = serving && !fx.serving;
    fx.serving = serving;
    if serve {
        fx.rng = Lcg(fx.seed);
        fx.camera = None;
        for wind in &mut fx.winds {
            wind.setup(&mut fx.rng, today.speed, today.weather);
        }
    }
    let rain = hst_sim::weather::rain(today.weather);
    if rain && !fx.rain {
        fx.camera = None;
    }
    fx.rain = rain;
    if rain {
        fx.rain(cam, today.degrees, today.speed);
        if (fx.tick / FRAME) % 4 == 0 {
            let z = CELL * fx.rng.r();
            fx.ground = Vec2::new(CELL * fx.rng.r(), z);
        }
    }
    for wind in &mut fx.winds {
        wind.tick(&mut fx.rng, today.degrees, today.speed, today.weather);
    }
    fx.tick = fx.tick.wrapping_add(1);
}

/// One rain voice: its play, bearing (whole degrees), which way it turns, and its step and turn-back counters.
#[derive(Clone, Copy)]
struct Voice {
    id: u64,
    angle: i32,
    back: bool,
    step: i32,
    left: i32,
}

impl Voice {
    /// A degree every 5 ticks; the first turn-back after 45 degrees, then every 90 (a ±45° sweep about the start).
    fn tick(&mut self) -> bool {
        self.step += 1;
        if self.step % 5 != 0 {
            return false;
        }
        self.step = 0;
        self.angle = (self.angle + if self.back { -1 } else { 1 }).rem_euclid(360);
        self.left -= 1;
        if self.left < 1 {
            (self.left, self.back) = (90, !self.back);
        }
        true
    }
}

#[test]
fn voice_sweeps_45_each_way() {
    let mut v = Voice { id: 0, angle: 90, back: false, step: 0, left: 45 };
    let angles: Vec<i32> = (0..5 * 180).filter_map(|_| v.tick().then_some(v.angle)).collect();
    assert_eq!((angles[0], angles[44], angles[45], angles[134], angles[179]), (91, 135, 134, 45, 90));
}

/// The rain voices: started when the rain starts (and again if they run out), stopped when it ends; each starts
/// turning a random way (bit 16 of a draw).
/// ponytail: the game draws that bit from the court's generator (shared with the NPCs, reseeded each point); here
/// from one of its own seeded from the effects seed, until the NPCs draw from the game's generators too (P3b).
fn ambience(fx: Option<Res<Particles>>, sound: Option<Res<crate::audio::Sound>>, bank: Option<Res<crate::audio::CourtBank>>, mut voices: Local<Vec<Voice>>, mut mt: Local<Option<hst_sim::weather::Mt>>) {
    let (Some(fx), Some(sound), Some(Some(bank))) = (fx, sound, bank.map(|b| b.0.clone())) else { return };
    if !fx.rain || voices.iter().all(|v| !sound.playing(v.id)) {
        voices.drain(..).for_each(|v| sound.stop(v.id));
    }
    let volume = 0x80;
    if fx.rain && voices.is_empty() {
        let mt = mt.get_or_insert_with(|| hst_sim::weather::Mt::new(fx.seed));
        let p = hst_sim::sound::Play { slot: 0, program: 1, key: 2, volume, speed: 1.0 };
        *voices = [90, 135, 225, 270].map(|angle| Voice { back: mt.next() >> 16 & 1 != 0, id: sound.play_toward(&bank, p, angle), angle, step: 0, left: 45 }).to_vec();
        return;
    }
    for v in voices.iter_mut() {
        if v.tick() {
            sound.turn(v.id, volume, v.angle);
        }
    }
}

#[derive(Default)]
struct Quads {
    pos: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    colour: Vec<[f32; 4]>,
    index: Vec<u32>,
}

impl Quads {
    /// Corners in strip order (v0 v1 v2 v3: two triangles).
    fn push(&mut self, at: [Vec3; 4], uv: [[f32; 2]; 4], alpha: f32) {
        let n = self.pos.len() as u32;
        self.pos.extend(at.map(|v| v.to_array()));
        self.uv.extend(uv);
        self.colour.extend([[1.0, 1.0, 1.0, alpha]; 4]);
        self.index.extend([n, n + 1, n + 2, n + 1, n + 2, n + 3]);
    }
}

fn draw(fx: Option<Res<Particles>>, w: Option<Res<Weather>>, cam: Query<&Transform, With<Orbit>>, mut meshes: ResMut<Assets<Mesh>>) {
    let (Some(fx), Some(w), Ok(cam)) = (fx, w, cam.single()) else { return };
    let today = w.today();
    let [right, .., eye] = basis(cam);
    let mut sets: Vec<Quads> = (0..fx.views.len()).map(|_| Quads::default()).collect();
    if fx.rain {
        let (a, b) = (right * 0.3 * FAR, fx.fall.normalize_or_zero() * 0.2 * FAR);
        for q in fx.streaks.map(|q| eye + (q - eye) * FAR) {
            sets[0].push([q + b - a, q + b + a, q - b - a, q - b + a], [[0.0, 0.0], [20.0, 0.0], [0.0, 0.8], [20.0, 0.8]], 0.5);
        }
        // each cell is 2 × 2 tiles, each showing the next of the texture's four frames; the right-hand tiles are
        // mirrored in U and the far ones in V, so the four meet edge to edge
        let base = fx.tick / FRAME;
        let half = CELL / 2.0;
        let origin = Vec3::new(fx.ground.x - CELL * CELLS[0] as f32 / 2.0, -0.01, fx.ground.y - CELL * CELLS[1] as f32 / 2.0);
        for (cx, cz, k) in (0..CELLS[1]).flat_map(|z| (0..CELLS[0]).flat_map(move |x| (0..4).map(move |k| (x, z, k)))) {
            let f = (base + k as u32) % 4;
            let (u, v) = ((f & 1) as f32 * 0.5, (f >> 1) as f32 * 0.5);
            let p = origin + Vec3::new(cx as f32 * CELL + (k & 1) as f32 * half, 0.0, cz as f32 * CELL + (k >> 1) as f32 * half);
            let (dx, dz) = (Vec3::X * half, Vec3::Z * half);
            let (u0, u1) = if k & 1 == 0 { (u, u + 0.5) } else { (u + 0.5, u) };
            let (v0, v1) = if k & 2 == 0 { (v, v + 0.5) } else { (v + 0.5, v) };
            sets[1].push([p, p + dx, p + dz, p + dx + dz], [[u0, v0], [u1, v0], [u0, v1], [u1, v1]], 0.5);
        }
    }
    let a = wrap_pi(today.degrees.to_radians());
    let a2 = wrap_pi(a + FRAC_PI_2);
    for wind in fx.winds.iter().filter(|w| w.on) {
        for l in wind.leaves.iter().filter(|l| l.live) {
            let mut m = if l.landed {
                mul(rot_z(PI), mul(mul(rot_x(-FRAC_PI_2), rot_y(-FRAC_PI_2)), rot_y(a)))
            } else {
                mul(mul(rot_y(l.spin), rot_x(-FRAC_PI_2)), rot_y(a2))
            };
            if l.flip {
                m = mul(rot_z(PI), m);
            }
            let (r0, r1, c) = (m[0] * HALF[wind.kind], m[1] * HALF[wind.kind], l.shown);
            sets[2 + wind.kind].push([c - r0 - r1, c + r0 - r1, c - r0 + r1, c + r0 + r1], [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]], l.alpha / 128.0);
        }
    }
    for ((mesh, _), q) in fx.views.iter().zip(sets) {
        if let Some(mut m) = meshes.get_mut(mesh) {
            fill(&mut m, q.pos, q.uv, q.colour, q.index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaf_lands_on_its_spot() {
        let mut rng = Lcg(7);
        let mut wind = Wind { kind: 0, strength: 2, speed: -1, ..default() };
        wind.setup(&mut rng, 4.0, 0);
        assert!(wind.on);
        wind.tick(&mut rng, 90.0, 4.0, 0);
        // the first leaf starts in mid-flight; tick it to its landing
        let first = wind.leaves[0];
        assert!(first.live && first.t > 300 && first.alpha == 0.0);
        while !wind.leaves[0].landed {
            wind.tick(&mut rng, 90.0, 4.0, 0);
        }
        let l = wind.leaves[0];
        assert!((l.pos.y + 0.01).abs() < 1e-3 && l.pos.x.abs() <= 9.01 && l.pos.z.abs() <= 15.01, "{:?}", l.pos);
        // calm or rain: no leaves
        wind.setup(&mut rng, 2.0, 0);
        assert!(!wind.on);
        wind.setup(&mut rng, 9.0, 2);
        assert!(!wind.on);
    }
}
