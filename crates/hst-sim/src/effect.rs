//! Effect models (`AZUMA/C_EFF/EFFCT.XB0`): a model played by key channels, each on its own clock — node
//! animation (`.ANI`, a hierarchy under a scene root), morph-target weights (`.MOR`) and material alpha (`.MTA`,
//! tracks named after the materials they drive). Starting an effect sets every clock to 0 and plays its first frame; each
//! frame advances them as a motion clock ([`Clock`], speed 1, clamped), the model's nodes first, then the
//! morphs, then the alphas. The effect ends the frame the morph clock's sampled time reaches the morph length (the
//! last key of its bound tracks, in frames): the frames before that are drawn.
//!
//! `.UVA` files sit beside some effects but never play: the game loads them into the alpha channel as a second
//! clip it never selects.

use hst_data::{ani::Anim, mdl::Model, mor::Tracks, mtl::Material};

use crate::{
    face,
    motion::Clock,
    pose::{Clip, M4, Skeleton},
    ps2,
};

struct Track {
    /// Morph target or material index.
    target: usize,
    ticks: Vec<i32>,
    values: Vec<[f32; 4]>,
    cursor: usize,
}

struct Channel {
    tracks: Vec<Track>,
    ticks_per_frame: f32,
    length: f32,
    clock: Clock,
}

impl Channel {
    /// Tracks whose name `bind` finds, in file order.
    fn new(t: &Tracks, bind: impl Fn(&str) -> Option<usize>) -> Channel {
        let tracks: Vec<Track> = t
            .tracks
            .iter()
            .filter_map(|tr| Some(Track { target: bind(&tr.name)?, ticks: tr.ticks.clone(), values: tr.values.clone(), cursor: 0 }))
            .collect();
        let length = face::length(tracks.iter().map(|t| &t.ticks[..]), t.ticks_per_frame);
        Channel { tracks, ticks_per_frame: t.ticks_per_frame as f32, length, clock: Clock::start(1.0, false, None) }
    }

    /// Sample every track at the clock's sampled time into `out[target]` (`empty` for a track without keys).
    fn apply(&mut self, out: &mut [f32], empty: f32) {
        let t = ps2::mul(self.clock.sampled, self.ticks_per_frame);
        for tr in &mut self.tracks {
            out[tr.target] = if tr.ticks.is_empty() { empty } else { face::sample(&tr.ticks, &tr.values, t, &mut tr.cursor, false)[0] };
        }
    }
}

pub struct Effect {
    pub skeleton: Skeleton,
    pub clip: Clip,
    model: Clock,
    morph: Channel,
    alpha: Channel,
    /// Per morph target of the model its weight.
    pub weights: Vec<f32>,
    /// Per material its alpha (1 = the MTL's 0x80): the `.MTA` value for driven materials, else the MTL's own.
    pub alphas: Vec<f32>,
    /// Started and not yet ended: drawn this frame.
    pub live: bool,
}

impl Effect {
    pub fn new(mdl: &Model, ani: &Anim, mor: &Tracks, mta: &Tracks, materials: &[Material]) -> Effect {
        let skeleton = Skeleton { names: mdl.node_names.clone(), parent: mdl.node_parent.clone(), rest: mdl.node_local.clone() };
        let clip = Clip::new(&skeleton, ani);
        let morph = Channel::new(mor, |n| mdl.morph_names.iter().position(|m| m == n));
        let alpha = Channel::new(mta, |n| materials.iter().position(|m| m.name == n));
        Effect {
            skeleton,
            clip,
            model: Clock::start(1.0, false, None),
            morph,
            alpha,
            weights: vec![0.0; mdl.morph_names.len()],
            alphas: materials.iter().map(|m| m.color[3]).collect(),
            live: false,
        }
    }

    /// Start (or restart) the effect: its first frame, sampled at time 0.
    pub fn start(&mut self) {
        self.live = true;
        self.set_zero();
        self.tick();
    }

    fn set_zero(&mut self) {
        for c in [&mut self.model, &mut self.morph.clock, &mut self.alpha.clock] {
            *c = Clock::start(1.0, false, None);
        }
        self.morph.apply(&mut self.weights, 0.0);
        self.alpha.apply(&mut self.alphas, 1.0);
    }

    /// One frame: advance every channel; past the morph length the effect resets to 0 and ends.
    pub fn tick(&mut self) {
        if !self.live {
            return;
        }
        self.model.tick(self.clip.length);
        self.morph.clock.tick(self.morph.length);
        self.morph.apply(&mut self.weights, 0.0);
        self.alpha.clock.tick(self.alpha.length);
        self.alpha.apply(&mut self.alphas, 1.0);
        if self.morph.clock.done(self.morph.length) {
            self.live = false;
            self.set_zero();
        }
    }

    /// The model clock's sampled time and the morph/alpha clocks' (frames).
    pub fn times(&self) -> [f32; 3] {
        [self.model.sampled, self.morph.clock.sampled, self.alpha.clock.sampled]
    }

    /// Every node's local matrix this frame.
    pub fn locals(&self) -> Vec<M4> {
        self.clip.locals(&self.skeleton, self.model.sampled)
    }
}

/// The racket-impact effect's scale for shot kind `kind` (0 top … 4 drop) from the ball's velocity as the shot
/// leaves: twice the speed for top and flat shots, 1 otherwise.
pub fn impact_scale(kind: i32, vel: [f32; 3]) -> f32 {
    use ps2::{add, mul};
    if kind != 0 && kind != 2 {
        return 1.0;
    }
    let [x, y, z] = vel;
    mul(ps2::sqrt(add(add(mul(x, x), mul(y, y)), mul(z, z))), 2.0)
}

/// The racket-impact effect's world matrix (rows x, y, z, translation): z along the ball's velocity `vel` (its w
/// scaled alike), x level (up × z), y = z × x, at the ball's position `pos`, fixed as the shot leaves.
pub fn impact_matrix(vel: [f32; 4], pos: [f32; 3]) -> [[f32; 4]; 4] {
    use ps2::{div, mul, sub};
    let len = |[x, y, z]: [f32; 3]| div(1.0, ps2::sqrt(ps2::madd(ps2::madd(mul(x, x), y, y), z, z)));
    let q = len([vel[0], vel[1], vel[2]]);
    let [x, y, z, w] = vel.map(|c| mul(c, q));
    let place = [pos[0], pos[1], pos[2], 1.0];
    if y.abs() >= 0.999999 {
        // ponytail: straight up/down never happens off a racket; the game turns a quarter turn about x
        let s = if y < 0.0 { 1.0 } else { -1.0 };
        return [[1.0, 0.0, 0.0, 0.0], [0.0, 0.0, s, 0.0], [0.0, -s, 0.0, 0.0], place];
    }
    let a = [sub(mul(z, 1.0), mul(y, 0.0)), sub(mul(x, 0.0), mul(z, 0.0)), sub(mul(y, 0.0), mul(x, 1.0))];
    let b = [sub(mul(y, a[2]), mul(z, a[1])), sub(mul(z, a[0]), mul(x, a[2])), sub(mul(x, a[1]), mul(y, a[0]))];
    let (qa, qb) = (len(a), len(b));
    [[mul(a[0], qa), mul(a[1], qa), mul(a[2], qa), mul(qa, 0.0)], [mul(b[0], qb), mul(b[1], qb), mul(b[2], qb), mul(qb, 0.0)], [x, y, z, w], place]
}
