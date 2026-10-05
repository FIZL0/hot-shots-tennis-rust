//! Ball flight. The original integrates in f32 with this exact operation order; keep it, rounding is gameplay.

pub type V3 = [f32; 3];

/// 9.8 m/s² expressed per 60 Hz frame².
pub const GRAVITY_PER_FRAME: f32 = 0.0027222224;

/// Per-shot constants that stay fixed for a whole flight.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// Quadratic drag coefficient (0.04 for the regular ball).
    pub drag: f32,
    /// Gravity multiplier (0.9 for the regular ball).
    pub gravity: f32,
    /// Magnus strength (0.0015).
    pub magnus: f32,
    /// Spin axis in world space; (0, 1, 0) for normal shots.
    pub axis: V3,
}

impl Default for Params {
    fn default() -> Self {
        Self { drag: 0.04, gravity: 0.9, magnus: 0.0015, axis: [0.0, 1.0, 0.0] }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub pos: V3,
    /// Displacement per frame.
    pub vel: V3,
    /// Signed spin rate; positive = topspin in the game's convention.
    pub spin: f32,
}

fn len(v: V3) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

impl Ball {
    /// Velocity update for one airborne frame: drag, Magnus, gravity. `dt` is the slow-motion scale (1 normally).
    pub fn accelerate(&mut self, p: &Params, dt: f32) {
        let v = self.vel.map(|c| c * dt);
        let s = len(v);
        if s == 0.0 {
            self.vel = v;
            return;
        }
        let inv = 1.0 / s;
        let v = v.map(|c| c - c * inv * s * s * p.drag);
        // lift acts along v × (axis × v), i.e. perpendicular to travel within the spin plane
        let w = cross(v, cross(p.axis, v));
        let wl = 1.0 / len(w);
        let mut v = [0, 1, 2].map(|i| w[i] * wl * s * self.spin * p.magnus + v[i]);
        v[1] += dt * p.gravity * GRAVITY_PER_FRAME;
        self.vel = v;
    }

    /// One airborne frame with no contact: semi-implicit Euler (velocity first, then position).
    pub fn fly(&mut self, p: &Params) {
        self.accelerate(p, 1.0);
        self.pos = [0, 1, 2].map(|i| self.pos[i] + self.vel[i]);
    }
}
