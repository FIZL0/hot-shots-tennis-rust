//! The ball's wind tornado (`wind2/tatumakiball`): a shot leaving the racket at 90 km/h or more starts it at the
//! ball. Its model scales up by the ball's launch speed a frame until 8 times that, then fades out over `fade`
//! frames; it rides the ball along its velocity and stops at the first bounce. Its UV animation runs at the launch
//! speed + 1.5. On the FPU (`ps2`), game space (Y-down).
//!
//! The game also has a flash quad for a grade-1 hit's tornado, but its size and colours are never set (zero), so it
//! draws nothing and isn't ported.

use crate::{libm, ps2, world};

type V4 = [f32; 4];

#[derive(Clone, Copy, Debug, Default)]
pub struct Tornado {
    pub on: bool,
    /// The model's scale.
    pub t: f32,
    /// The scale it grows to.
    pub end: f32,
    /// The ball's speed at the start (m per frame).
    pub speed: f32,
    /// Fade frames left.
    pub fade: i32,
    /// Alpha 0..128 while it fades (0 before: drawn opaque, see [`Tornado::opacity`]).
    pub alpha: f32,
    /// The model's matrix: z along the ball's flight, at the ball.
    pub m: world::M4,
}

impl Tornado {
    /// A shot leaves the racket at `vel`; `fade` is the game's fade length (frames).
    pub fn start(&mut self, vel: V4, fade: i32) {
        use ps2::{add, madd, mul};
        let speed = ps2::sqrt(madd(madd(mul(vel[0], vel[0]), vel[1], vel[1]), vel[2], vel[2]));
        let kmh = ps2::div(mul(mul(speed, 60.0), 3600.0), 1000.0);
        *self = Tornado { on: kmh >= 90.0, t: 0.0, end: add(add(mul(speed, 8.0), 0.0), 0.0), speed, fade, ..*self };
    }

    /// The UV animation's speed (frames a frame).
    pub fn uv_speed(&self) -> f32 {
        ps2::add(self.speed, 1.5)
    }

    /// The model's material alpha (1 opaque).
    pub fn opacity(&self) -> f32 {
        if self.end <= self.t { self.alpha / 128.0 } else { 1.0 }
    }

    /// A slow-motion in-between frame, `frac` (0..1] of the way from the last [`Tornado::tick`] to the next, as the
    /// game draws it: the ball at `pos` moving at `vel`, `bounces` so far; `uv` the UV animation's times before and
    /// after the last tick; `floor` the game's lowest alpha. Returns the drawn scale and UV time (`t` stays the
    /// tick's; the alpha is the in-between's).
    pub fn between(&mut self, bounces: i32, pos: V4, vel: V4, frac: f32, uv: [f32; 2], fade: i32, floor: i32) -> (f32, f32) {
        use ps2::{add, div, madd, mul, sub};
        let mut scale = self.t;
        if self.t < self.end {
            scale = madd(add(0.0, self.t), self.speed, frac);
        }
        let uv = madd(add(0.0, uv[0]), frac, sub(uv[1], uv[0]));
        if !self.on {
            return (scale, uv);
        }
        if bounces >= 1 {
            self.on = false;
            return (scale, uv);
        }
        self.aim(pos, vel);
        if self.end <= scale {
            let a = div(mul(128.0, sub(self.fade as f32, frac)), fade as f32);
            self.alpha = if a < floor as f32 { floor as f32 } else { a };
        }
        (scale, uv)
    }

    fn aim(&mut self, pos: V4, vel: V4) {
        use ps2::{add, madd, mul, sub};
        let d: V4 = std::array::from_fn(|k| sub(add(pos[k], vel[k]), pos[k]));
        let pitch = libm::atan2f(-d[1], ps2::sqrt(madd(mul(d[0], d[0]), d[2], d[2])));
        self.m = world::mat_mul(&world::rot_x(pitch), &world::rot_y(libm::atan2f(d[0], d[2])));
        self.m[3] = pos;
    }

    /// One frame: the ball at `pos` moving at `vel`, `bounces` so far; `fade` as at the start.
    pub fn tick(&mut self, bounces: i32, pos: V4, vel: V4, fade: i32) {
        use ps2::add;
        if !self.on {
            return;
        }
        if self.t < self.end {
            self.t = add(self.t, self.speed);
        }
        if bounces >= 1 {
            self.on = false;
            return;
        }
        self.aim(pos, vel);
        if self.end <= self.t {
            self.alpha = ((self.fade << 7) / fade) as f32;
            self.fade -= 1;
            self.on = self.fade >= 0;
        }
    }
}
