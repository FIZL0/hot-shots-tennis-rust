//! Footsteps: a running player's toes (`Bip01RToe0`, `Bip01LToe0`) raise a puff of dust (dry dusty courts) or
//! spray (rain), and leave footprints on courts that keep them. A foot steps when, lifted more than 0.08 m, it
//! comes back down to the character's `lift` height with its cooldown run out; only motions the character's table
//! marks as running are watched. Puffs fade in then out while they grow, rise, slide on along the step and drift
//! with the wind; footprints fade out over the court's life. Two more kinds of puff: a burst of four where the hips
//! (`Bip01Pelvis`) come down to the court in a reaction that sits the player down (losing a game: `di_set`, some
//! characters' `di` or `gu_set`; once a point), and a ring of ten thrown out as a dive
//! starts, which also shows the `run/dash` streak model at the hips while the dive lasts. In an instant replay a dive
//! also throws a group of 20 bits ahead of the right toe: clods on clay courts, grass blades on grass (dry) ones; they
//! fly, fall and shrink, the blades spinning and drifting with the wind. On the FPU (`ps2`), game space (Y-down).

use hst_data::exe::Foot;

use crate::{effect, libm, ps2, shot, weather, world};

type V4 = [f32; 4];

/// Most puffs alive at once (the oldest goes).
pub const PUFFS: usize = 100;
/// Most footprints at once (the oldest goes).
pub const PRINTS: usize = 40;

/// Debris groups alive at once (a fourth dive drops the oldest) and bits per group.
pub const GROUPS: usize = 3;
pub const BITS: usize = 20;

/// A replay dive's clod or blade: at `m[3]`, `size` half wide, texture cell `cell` (pixels). A blade lies in `m`'s
/// rows 0 and 2, turned by `spin` (x, y, z radians) a frame, coloured `rgb`; a clod faces the camera (its rows 0..2,
/// spin and colour are stale).
#[derive(Clone, Copy, Debug, Default)]
pub struct Bit {
    pub alive: bool,
    pub m: world::M4,
    pub vel: V4,
    pub size: f32,
    pub cell: [i32; 2],
    pub spin: V4,
    pub rgb: [f32; 3],
}

/// A camera-facing puff, anchored at its bottom: `pos`, `size` half wide and twice that tall, `alpha` 0..128.
#[derive(Clone, Copy, Debug, Default)]
pub struct Puff {
    /// 0 a step, 1 a dive ring, 2 a sit-down burst.
    pub kind: u8,
    /// 0 fading in, 1 fading out.
    pub phase: i32,
    pub timer: i32,
    pub size: f32,
    pub grow: f32,
    pub alpha: f32,
    pub fade: f32,
    pub pos: V4,
    /// Unit level direction it slides along (from where it rose to the foot a frame later), or 0.
    pub dir: V4,
    pub foot: usize,
    pub player: usize,
    /// The direction is still to be set.
    pub aim: bool,
    pub speed: f32,
    /// Born this frame: left as is.
    pub fresh: bool,
    /// The motion's puff scale (dusty courts).
    pub scale: f32,
    /// A dive ring's outward step per frame.
    pub vel: V4,
}

/// A footprint: a ground quad turned with the player, 0.2 m across `m[0]` and 0.4 m along `m[2]`, at `m[3]`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Print {
    pub m: world::M4,
    pub alpha: f32,
    pub life: i32,
    pub fade: f32,
}

/// One player this frame.
#[derive(Default)]
pub struct Runner {
    pub character: usize,
    pub motion: i32,
    /// The motion's sub-state (past the point, a late enough part of a high motion takes no steps).
    pub sub: i32,
    /// The right and left toe bones' world positions.
    pub toes: [V4; 2],
    /// The player's world matrix.
    pub m: world::M4,
    /// A puff slides along the step (else it stays put).
    pub slide: bool,
    /// The `Bip01Pelvis` and `Bip01Spine1` world matrices and the `Bip01Head` position.
    pub pelvis: world::M4,
    pub spine: world::M4,
    pub head: V4,
    /// A dive started this frame, and its slide length.
    pub dive: bool,
    pub lunge: f32,
    /// The dive's streak stops (the player's dive is done).
    pub dive_over: bool,
    /// Where the current motion set off (a dive's debris flies on from there through the player).
    pub anchor: V4,
}

#[derive(Clone, Default)]
pub struct Feet {
    pub armed: [[bool; 2]; 4],
    pub cooldown: [[i32; 2]; 4],
    pub puffs: Vec<Puff>,
    pub prints: Vec<Print>,
    /// The point is over.
    pub after_point: bool,
    /// The player's sit-down burst is spent for the point.
    pub burst: [bool; 4],
    /// The `run/dash` streak showing (yaw of the hips at the dive's start, at the hips) and restarted this frame.
    pub dash: [Option<world::M4>; 4],
    pub dash_start: [bool; 4],
    /// The foot landed this frame (the replay's footstep sound, `sound::step`).
    pub stepped: [[bool; 2]; 4],
    /// An instant replay is showing: dives throw debris.
    pub replay: bool,
    pub bits: [[Bit; BITS]; GROUPS],
    pub groups: usize,
    /// Each group's kind (−1 none, 0 clods, 1 blades), filed for every dive before its group is made, so with three
    /// groups up it lands past the end and a new group reads its predecessor's. Group 0's picks every group's look.
    // ponytail: the game's fourth entry overwrites the first streak matrix's x; that one never shows a frame later
    pub kinds: [i32; GROUPS + 1],
    /// The C library's `rand()`, drawn to spin blades.
    pub crand: weather::Rand,
}

impl Feet {
    /// One frame on `court`: steps (when `stepping`), then every puff and footprint ages. `dusty` and `wet` pick the
    /// puff (dust, spray, none) and footprint looks; `wind` is the weather's wind vector; `rand` the game's shared
    /// generator (`weather::Mt`), drawn twice per dive-ring puff.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(&mut self, t: &Foot, court: usize, dusty: bool, wet: bool, wind: V4, runners: &[Runner], stepping: bool, rand: &mut impl FnMut() -> u32) {
        use ps2::{add, div, madd, mul, sub};
        let row = t.puffs[!dusty as usize];
        let c = t.courts[court];
        self.dash_start = [false; 4];
        self.stepped = [[false; 2]; 4];
        for (p, r) in runners.iter().enumerate().filter(|_| stepping) {
            let st = &t.steps[r.character];
            let scale = st.scale.get(r.motion as usize).copied().unwrap_or(0.0);
            // the hips (one character's spine in one motion) coming down to the court
            let low = |m: &world::M4, y: f32| m[3][1] >= y;
            if st.burst.get(r.motion as usize).copied().unwrap_or(false) && !self.burst[p] && low(&r.pelvis, -0.25) {
                self.burst[p] = true;
                self.spray(t, dusty, wet, scale, &r.pelvis);
            }
            if r.character == 0xd && r.motion == 0x2f && !self.burst[p] && low(&r.spine, -0.26) {
                self.burst[p] = true;
                self.spray(t, dusty, wet, scale, &r.spine);
            }
            let late = self.after_point && r.motion >= 0x2c && r.sub >= 0x30;
            let runs = !late && st.runs.get(r.motion as usize).copied().unwrap_or(false);
            for f in (0..2).filter(|_| runs) {
                let y = r.toes[f][1];
                let mut step = false;
                if !self.armed[p][f] {
                    self.armed[p][f] = y < -0.08;
                } else if st.lift <= y && self.cooldown[p][f] < 1 {
                    self.armed[p][f] = false;
                    step = true;
                    self.cooldown[p][f] = st.cooldown;
                    self.stepped[p][f] = true;
                }
                if step && (dusty || wet) {
                    let by = |v: f32| if dusty { mul(v, scale) } else { v };
                    let mut pos = r.toes[f];
                    pos[1] = f32::from_bits(0x3da3_d70a);
                    self.push(Puff {
                        timer: row.fade_in,
                        size: row.size,
                        grow: div(by(row.grow), (row.fade_in + row.fade_out) as f32),
                        fade: div(by(row.alpha as f32), row.fade_in as f32),
                        pos,
                        foot: f,
                        player: p,
                        aim: true,
                        speed: row.speed,
                        fresh: true,
                        scale,
                        ..Puff::default()
                    });
                }
                if step && c.prints {
                    if self.prints.len() >= PRINTS {
                        self.prints.remove(0);
                    }
                    let m = r.m;
                    let k = mul(m[1][2], m[2][1]);
                    let mut pm = world::rot_y(if k == 1.0 || k == -1.0 { 0.0 } else { libm::atan2f(m[2][0], m[2][2]) });
                    pm[3] = r.toes[f];
                    pm[3][1] = f32::from_bits(0xbc23_d70a);
                    let alpha = c.print_alpha[wet as usize] as f32;
                    let life = c.print_life[wet as usize];
                    self.prints.push(Print { m: pm, alpha, life, fade: -div(alpha, life as f32) });
                }
                if self.cooldown[p][f] > 0 {
                    self.cooldown[p][f] -= 1;
                }
            }
            if r.dive {
                if r.lunge > 0.5 {
                    let m = r.pelvis;
                    let k = mul(m[1][2], m[2][1]);
                    let mut d = world::rot_y(if k == 1.0 || k == -1.0 { 0.0 } else { libm::atan2f(m[2][0], m[2][2]) });
                    d[3] = m[3];
                    self.dash[p] = Some(d);
                    self.dash_start[p] = true;
                }
                if dusty || wet {
                    // out from the player, pushed toward the head (away from it in rain)
                    let (dx, dz) = (sub(r.head[0], r.m[3][0]), sub(r.head[2], r.m[3][2]));
                    let q = div(1.0, ps2::sqrt(madd(mul(dz, dz), dx, dx)));
                    let s = if wet { -1.0 } else { 1.0 };
                    // the game keeps y +0 but scales w, so w is −0 in rain
                    let dir = [mul(mul(dx, q), s), 0.0, mul(mul(dz, q), s), mul(0.0, s)];
                    let draw = |rand: &mut dyn FnMut() -> u32, [lo, hi]: [f32; 2]| madd(add(0.0, lo), sub(hi, lo), mul(f32::from_bits(0x2f80_0000), ps2::utof(rand())));
                    let life = (row.fade_in + row.fade_out2) as f32;
                    for k in 0..10 {
                        let size = draw(rand, row.ring_size);
                        let mut u = Puff {
                            kind: 1,
                            timer: row.fade_in,
                            size,
                            grow: div(mul(2.0, size), life),
                            fade: div(row.alpha2 as f32, row.fade_in as f32),
                            pos: [r.m[3][0], f32::from_bits(0x3da3_d70a), r.m[3][2], r.m[3][3]],
                            dir,
                            player: p,
                            speed: draw(rand, row.ring_speed),
                            fresh: true,
                            ..Puff::default()
                        };
                        let a = mul(f32::from_bits(0x3c8e_fa35), mul(36.0, k as f32));
                        let v = mul(u.speed, life);
                        u.vel = [div(mul(v, libm::cosf(a)), life), 0.0, div(mul(v, libm::sinf(a)), life), 0.0];
                        self.push(u);
                    }
                }
                let kind = if c.grass { 1 } else if c.clay { 0 } else { -1 };
                self.kinds[self.groups] = kind;
                if self.replay && (kind == 0 || kind == 1 && !wet) {
                    self.throw(t, court, kind as usize, r, rand);
                }
            }
            if self.dash[p].is_some() && (r.motion != 0x1e || r.dive_over) {
                self.dash[p] = None;
            }
        }

        let after_point = self.after_point;
        let by = |a: i32, u: &Puff| if dusty { mul(a as f32, u.scale) } else { a as f32 };
        self.puffs.retain_mut(|u| {
            if u.kind != 2 && std::mem::take(&mut u.fresh) {
                return true;
            }
            let drift = |u: &mut Puff| u.pos = std::array::from_fn(|k| madd(add(0.0, u.pos[k]), wind[k], t.wind));
            let (out, from) = match u.kind {
                1 => {
                    drift(u);
                    u.size = add(u.size, u.grow);
                    u.pos = std::array::from_fn(|k| add(u.pos[k], u.vel[k]));
                    u.pos = std::array::from_fn(|k| madd(add(0.0, u.pos[k]), u.dir[k], u.speed));
                    u.pos[1] = add(u.pos[1], row.ring_rise);
                    (row.fade_out2, row.alpha2 as f32)
                }
                2 => {
                    u.size = add(u.size, u.grow);
                    u.pos[1] = add(u.pos[1], row.rise);
                    (row.fade_out2, by(row.alpha2, u))
                }
                _ => {
                    step_age(u, runners, after_point, row.rise, t.decay, &drift);
                    (row.fade_out, by(row.alpha, u))
                }
            };
            u.alpha = add(u.alpha, u.fade);
            u.timer -= 1;
            if u.timer >= 1 {
                return true;
            }
            if u.phase == 1 {
                return false;
            }
            u.phase = 1;
            u.timer = out;
            u.fade = -div(from, out as f32);
            true
        });
        self.prints.retain_mut(|p| {
            p.alpha = add(p.alpha, p.fade);
            p.life -= 1;
            p.life >= 0
        });
        self.age_bits(t, wind);
    }

    /// A new group of bits of `kind` (0 clods, 1 blades) out of `r`'s dive: thrown ahead of the right toe, along
    /// the way from where the motion set off through the player, up to the row's `reach` and `spread` around it.
    fn throw(&mut self, t: &Foot, court: usize, kind: usize, r: &Runner, rand: &mut impl FnMut() -> u32) {
        use ps2::{add, div, madd, mul, sub};
        let lunge = (r.lunge as f64 * 0.3) as f32;
        let (dx, dz) = (sub(r.m[3][0], r.anchor[0]), sub(r.m[3][2], r.anchor[2]));
        let q = div(1.0, ps2::sqrt(madd(mul(dx, dx), dz, dz)));
        let dir = [mul(mul(dx, q), lunge), 0.08, mul(mul(dz, q), lunge), mul(lunge, 0.0)];
        let o = [r.toes[0][0], -0.05, r.toes[0][2], r.toes[0][3]];
        if self.groups > 2 {
            self.drop_group();
        }
        self.groups += 1;
        let g = self.groups - 1;
        let row = t.debris.rows[self.kinds[g] as usize];
        let cells = t.debris.cells[kind];
        let b = effect::impact_matrix(dir, [0.0; 3]);
        let mut basis = [b[0], b[1], b[2], std::array::from_fn(|k| madd(add(0.0, o[k]), b[2][k], row.reach))];
        let unit = |x: u32| mul(ps2::utof(x), f32::from_bits(0x2f80_0000));
        let span = |[lo, hi]: [f32; 2], x: u32| madd(add(0.0, lo), mul(sub(hi, lo), ps2::utof(x)), f32::from_bits(0x2f80_0000));
        let size = |r: f32| div(madd(add(0.0, row.size[0]), sub(1.0, r), sub(row.size[1], row.size[0])), 2.0);
        let rgb = t.courts[court].blade_rgb;
        for k in 0..BITS {
            // the throw turns about the way out by a random angle per bit (blades only spend the draw)
            basis = world::mat_mul(&shot::rot_z(mul(unit(rand()), f32::from_bits(0x40c9_0fdb))), &basis);
            let s = &mut self.bits[g][k];
            s.alive = true;
            if kind == 0 {
                let r = unit(rand());
                let d: V4 = std::array::from_fn(|i| sub(madd(add(0.0, basis[3][i]), mul(basis[0][i], r), row.spread), o[i]));
                let q = div(1.0, ps2::sqrt(madd(madd(mul(d[1], d[1]), d[0], d[0]), d[2], d[2])));
                let mut m = ps2::sqrt(madd(mul(dir[2], dir[2]), dir[0], dir[0]));
                let [lo, hi] = row.speed;
                let sp = madd(add(0.0, lo), sub(hi, lo), unit(rand()));
                if row.power > 0.0 {
                    m = libm::powf(m, row.power);
                }
                let m = mul(m, sp);
                s.vel = d.map(|c| mul(mul(c, q), m));
                let y = if s.vel[1] > 0.0 { -s.vel[1] } else { s.vel[1] };
                s.vel[1] = if t.debris.lift <= y { y } else { t.debris.lift };
                s.size = size(r);
                s.m[3] = o;
            } else {
                s.m = world::IDENTITY;
                s.size = size(unit(rand()));
                s.vel = dir.map(|c| mul(c, row.push));
                s.vel[1] = 0.0;
            }
            s.cell = cells[(rand() >> 16 & 3) as usize];
            if kind == 1 {
                let deg = f32::from_bits(0x3c8e_fa35);
                let mut turn = |[lo, hi]: [f32; 2]| {
                    let (lo, hi) = (mul(lo, deg), mul(hi, deg));
                    madd(add(0.0, lo), mul(sub(hi, lo), f32::from_bits(0x3000_0000)), (self.crand.next() & 0xffff_f800) as i32 as f32)
                };
                let z = turn(row.spin[2]);
                let y = turn(row.spin[1]);
                let x = turn(row.spin[0]);
                let s = &mut self.bits[g][k];
                s.spin = [x, y, z, 0.0];
                s.m = world::mat_mul(&euler(s.spin), &s.m);
                s.m[3] = o;
                s.m[3][0] = add(s.m[3][0], span(row.offset[0], rand()));
                s.m[3][1] = add(s.m[3][1], span(row.offset[1], rand()));
                s.rgb = rgb[(k >= 11) as usize];
            }
        }
    }

    /// Every group's bits move on a frame: clods fly and fall until they reach the court, blades drift with half
    /// the `wind`, sink and spin until they reach it or shrink away. A group with nothing left drops the oldest
    /// group (not itself, as the game does).
    fn age_bits(&mut self, t: &Foot, wind: V4) {
        use ps2::{add, madd, mul};
        let mut i = 0;
        while i < self.groups {
            let kind = self.kinds[i];
            if kind != 0 && kind != 1 {
                i += 1;
                continue;
            }
            let row = t.debris.rows[kind as usize];
            let mut any = false;
            for s in self.bits[i].iter_mut().filter(|s| s.alive) {
                any = true;
                if kind == 0 {
                    s.m[3] = std::array::from_fn(|k| add(s.m[3][k], s.vel[k]));
                    s.alive = s.m[3][1] < 0.0;
                    s.vel = s.vel.map(|v| mul(v, row.decay));
                    s.vel[1] = add(s.vel[1], mul(row.fall, f32::from_bits(0x3b32_6750)));
                } else {
                    s.m[3] = std::array::from_fn(|k| add(s.m[3][k], madd(s.vel[k], wind[k], 0.5)));
                    s.m[3][1] = add(s.m[3][1], t.debris.sink);
                    s.vel = s.vel.map(|v| mul(v, row.decay));
                }
                s.size = mul(s.size, row.shrink);
                if kind == 1 {
                    s.m = world::mat_mul(&euler(s.spin), &s.m);
                    s.alive = !(s.m[3][1] >= 0.0 || s.size < 0.01);
                }
            }
            if any {
                i += 1;
            } else {
                self.drop_group();
            }
        }
    }

    /// The oldest debris group goes (the kinds stay put).
    fn drop_group(&mut self) {
        self.bits.copy_within(1..self.groups, 0);
        self.groups -= 1;
    }

    fn push(&mut self, u: Puff) {
        if self.puffs.len() >= PUFFS {
            self.puffs.remove(0);
        }
        self.puffs.push(u);
    }

    /// A sit-down burst of four at `m`: at the hips, 0.2 m ahead, and that either side across the court.
    fn spray(&mut self, t: &Foot, dusty: bool, wet: bool, scale: f32, m: &world::M4) {
        use ps2::{add, div, madd, mul};
        if !dusty && !wet {
            return;
        }
        let row = t.puffs[!dusty as usize];
        let by = |v: f32| if dusty { mul(v, scale) } else { v };
        for k in 0..4 {
            let mut pos = m[3];
            if k > 0 {
                pos = std::array::from_fn(|i| madd(add(0.0, m[3][i]), m[2][i], f32::from_bits(0x3e4c_cccd)));
            }
            if k == 1 || k == 2 {
                pos[0] = add(pos[0], f32::from_bits(if k == 1 { 0x3e19_999a } else { 0xbe19_999a }));
            }
            pos[1] = f32::from_bits(0x3da3_d70a);
            self.push(Puff {
                kind: 2,
                timer: row.fade_in,
                size: row.size,
                grow: div(by(row.grow), (row.fade_in + row.fade_out2) as f32),
                fade: div(by(row.alpha2 as f32), row.fade_in as f32),
                pos,
                scale,
                ..Puff::default()
            });
        }
    }
}

/// A turn by x, y and z (radians) about z, then y, then x.
fn euler([x, y, z, _]: V4) -> world::M4 {
    world::mat_mul(&world::mat_mul(&world::mat_mul(&world::IDENTITY, &shot::rot_z(z)), &world::rot_y(y)), &world::rot_x(x))
}

/// A step puff's motion: aimed along the step a frame after it rose, it grows, rises, slides on (slowing) and drifts.
fn step_age(u: &mut Puff, runners: &[Runner], after_point: bool, rise: f32, decay: f32, drift: &dyn Fn(&mut Puff)) {
    use ps2::{add, div, madd, mul, sub};
    if std::mem::take(&mut u.aim) {
        let r = &runners[u.player];
        u.dir = [0.0; 4];
        if r.slide && !after_point {
            let d: V4 = std::array::from_fn(|k| sub(r.toes[u.foot][k], u.pos[k]));
            let q = div(1.0, ps2::sqrt(madd(mul(d[0], d[0]), d[2], d[2])));
            u.dir = [mul(d[0], q), 0.0, mul(d[2], q), 0.0];
        }
    }
    u.size = add(u.size, u.grow);
    u.pos[1] = add(u.pos[1], rise);
    u.pos = std::array::from_fn(|k| madd(add(0.0, u.pos[k]), u.dir[k], u.speed));
    drift(u);
    u.speed = mul(u.speed, decay);
}
