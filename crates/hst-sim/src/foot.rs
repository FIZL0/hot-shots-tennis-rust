//! Footsteps: a running player's toes (`Bip01RToe0`, `Bip01LToe0`) raise a puff of dust (dry dusty courts) or
//! spray (rain), and leave footprints on courts that keep them. A foot steps when, lifted more than 0.08 m, it
//! comes back down to the character's `lift` height with its cooldown run out; only motions the character's table
//! marks as running are watched. Puffs fade in then out while they grow, rise, slide on along the step and drift
//! with the wind; footprints fade out over the court's life. On the FPU (`ps2`), game space (Y-down).

use hst_data::exe::Foot;

use crate::{libm, ps2, world};

type V4 = [f32; 4];

/// Most puffs alive at once (the oldest goes).
pub const PUFFS: usize = 100;
/// Most footprints at once (the oldest goes).
pub const PRINTS: usize = 40;

/// A camera-facing puff, anchored at its bottom: `pos`, `size` half wide and twice that tall, `alpha` 0..128.
#[derive(Clone, Copy, Debug, Default)]
pub struct Puff {
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
}

#[derive(Clone, Default)]
pub struct Feet {
    pub armed: [[bool; 2]; 4],
    pub cooldown: [[i32; 2]; 4],
    pub puffs: Vec<Puff>,
    pub prints: Vec<Print>,
    /// The point is over.
    pub after_point: bool,
}

impl Feet {
    /// One frame on `court`: steps (when `stepping`), then every puff and footprint ages. `dusty` and `wet` pick the
    /// puff (dust, spray, none) and footprint looks; `wind` is the weather's wind vector.
    pub fn tick(&mut self, t: &Foot, court: usize, dusty: bool, wet: bool, wind: V4, runners: &[Runner], stepping: bool) {
        use ps2::{add, div, madd, mul, sub};
        let row = t.puffs[!dusty as usize];
        let c = t.courts[court];
        for (p, r) in runners.iter().enumerate().filter(|_| stepping) {
            let st = &t.steps[r.character];
            let late = self.after_point && r.motion >= 0x2c && r.sub >= 0x30;
            if late || !st.runs.get(r.motion as usize).copied().unwrap_or(false) {
                continue;
            }
            for f in 0..2 {
                let y = r.toes[f][1];
                let mut step = false;
                if !self.armed[p][f] {
                    self.armed[p][f] = y < -0.08;
                } else if st.lift <= y && self.cooldown[p][f] < 1 {
                    self.armed[p][f] = false;
                    step = true;
                    self.cooldown[p][f] = st.cooldown;
                }
                if step && (dusty || wet) {
                    if self.puffs.len() >= PUFFS {
                        self.puffs.remove(0);
                    }
                    let scale = st.scale.get(r.motion as usize).copied().unwrap_or(0.0);
                    let by = |v: f32| if dusty { mul(v, scale) } else { v };
                    let mut pos = r.toes[f];
                    pos[1] = f32::from_bits(0x3da3_d70a);
                    self.puffs.push(Puff {
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
        }

        let after_point = self.after_point;
        self.puffs.retain_mut(|u| {
            if std::mem::take(&mut u.fresh) {
                return true;
            }
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
            u.pos[1] = add(u.pos[1], row.rise);
            u.pos = std::array::from_fn(|k| madd(add(0.0, u.pos[k]), u.dir[k], u.speed));
            u.pos = std::array::from_fn(|k| madd(add(0.0, u.pos[k]), wind[k], t.wind));
            u.speed = mul(u.speed, t.decay);
            u.alpha = add(u.alpha, u.fade);
            u.timer -= 1;
            if u.timer >= 1 {
                return true;
            }
            if u.phase == 1 {
                return false;
            }
            u.phase = 1;
            u.timer = row.fade_out;
            let a = row.alpha as f32;
            u.fade = -div(if dusty { mul(a, u.scale) } else { a }, row.fade_out as f32);
            true
        });
        self.prints.retain_mut(|p| {
            p.alpha = add(p.alpha, p.fade);
            p.life -= 1;
            p.life >= 0
        });
    }
}
