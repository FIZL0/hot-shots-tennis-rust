//! The motion numbers the game sets outside standing and running (`player`): strokes (wind-up turn, the swing
//! timed to contact, the soft follow-through, whiffs), the serve, and the reactions after a point.

/// A player's motion clock (its motion player's time). The motion setter restarts it at 0 with the speed; each
/// frame, after the player's update, the player samples at the time wrapped into the clip (looping) or clamped
/// (else), keeps that as `sampled`, then adds the speed — so `time` is a frame ahead of the pose. While a
/// crossfade holds the new motion (`hold`, the soft follow-through) the time stays put and the countdown runs;
/// the frame it passes zero the speed is added once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clock {
    pub time: f32,
    pub sampled: f32,
    pub speed: f32,
    pub looping: bool,
    pub hold: Option<i32>,
}

impl Clock {
    pub fn start(speed: f32, looping: bool, hold: Option<i32>) -> Clock {
        Clock { time: 0.0, sampled: 0.0, speed, looping, hold }
    }

    /// One frame of a clip `length` frames long.
    pub fn tick(&mut self, length: f32) {
        match self.hold {
            None => {
                self.time = crate::pose::wrap(self.time, length, self.looping);
                self.sampled = self.time;
                self.time = crate::ps2::add(self.time, self.speed);
            }
            Some(n) if n < 1 => {
                self.hold = None;
                self.time = crate::ps2::add(self.time, self.speed);
            }
            Some(n) => self.hold = Some(n - 1),
        }
    }

    /// The motion has played to its end (the game's end-of-motion test, on the sampled time).
    pub fn done(&self, length: f32) -> bool {
        length <= self.sampled
    }
}

/// The motion player's crossfade from the outgoing motion. The motion setter starts it with a length in frames
/// (8 for looping motions and the whiffs 0x27/0x28, the soft follow-through's own count with `hold`, a cut below
/// 2). Each frame, without `hold`, the outgoing motion keeps playing on its own clock and is mixed over the new pose
/// with `weight`, from (n−1)/n down to 0; with `hold`, its pose stays frozen at the last sampled time while the new
/// motion, held at frame 0, is mixed in with `weight` rising to 1 (the new clock's `hold` counts the same frames).
/// A switch mid-fade keeps whichever pose is more than half in and rescales the countdown to the new length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fade {
    /// The outgoing motion (−1 none) and whether it has a clip to sample.
    pub id: i32,
    pub clip: bool,
    pub sampled: f32,
    pub time: f32,
    pub speed: f32,
    pub looping: bool,
    pub frames: i32,
    pub count: i32,
    pub weight: f32,
    pub hold: bool,
}

impl Default for Fade {
    fn default() -> Fade {
        Fade { id: -1, clip: false, sampled: 0.0, time: 0.0, speed: 0.0, looping: false, frames: 0, count: 0, weight: 0.0, hold: false }
    }
}

impl Fade {
    /// The setter switching from motion `cur_id` (clock `cur`, `cur_clip` whether it has a clip) to `id`.
    pub fn start(&mut self, frames: i32, id: i32, hold: bool, cur_id: i32, cur_clip: bool, cur: &Clock) {
        use crate::ps2::{div, madd, sub};
        let ratio = |a: i32, b: i32| div(a as f32, b as f32);
        let cut = |f: &mut Fade| (f.id, f.clip) = (-1, false);
        let take = |f: &mut Fade| (f.id, f.clip, f.time, f.speed, f.looping) = (cur_id, cur_clip, cur.time, cur.speed, cur.looping);
        if frames < 2 {
            cut(self);
            self.hold = false;
            return;
        }
        if !self.clip {
            take(self);
            (self.frames, self.count) = (frames, frames - 1);
        } else {
            let f = ratio(self.count, self.frames);
            self.frames = frames;
            if 0.5 < f && self.id != id {
                cut(self);
            } else {
                take(self);
                // truncated toward zero, as cvt.w.s
                self.count = madd(0.5, if 0.5 <= f { f } else { sub(1.0, f) }, frames as f32) as i32;
            }
            self.count -= 1;
        }
        self.hold = hold;
        self.weight = ratio(self.count, self.frames);
        if hold {
            self.time = sub(self.time, self.speed);
            self.weight = sub(1.0, self.weight);
        }
        self.sampled = self.time;
    }

    /// The outgoing motion's part of a frame (before the player's update): its sampled time and the weight its
    /// pose is mixed with this frame (`None`: no fade). `length` is the outgoing clip's.
    pub fn tick(&mut self, length: f32) -> Option<f32> {
        if self.hold {
            if self.clip {
                self.time = crate::pose::wrap(self.time, length, self.looping);
                self.sampled = self.time;
            }
            return None;
        }
        let w = self.clip.then_some(self.weight);
        if self.clip {
            self.time = crate::pose::wrap(self.time, length, self.looping);
            self.sampled = self.time;
        }
        self.time = crate::ps2::add(self.time, self.speed);
        self.count -= 1;
        if self.count < 0 {
            (self.id, self.clip) = (-1, false);
        } else {
            self.weight = crate::ps2::div(self.count as f32, self.frames as f32);
        }
        w
    }

    /// The held fade's part of a frame (after the player's update): the weight the new motion's frame 0 is mixed
    /// over the frozen pose with (`None`: not holding).
    pub fn tick_hold(&mut self) -> Option<f32> {
        if !self.hold {
            return None;
        }
        let w = self.weight;
        self.count -= 1;
        if self.count < 0 {
            (self.id, self.clip, self.hold) = (-1, false, false);
        } else {
            self.weight = crate::ps2::sub(1.0, crate::ps2::div(self.count as f32, self.frames as f32));
        }
        Some(w)
    }
}

/// One track of the fade's mix: the current rotation and position toward the other pose's by `w` (shortest-arc
/// slerp; `old` positions taken from `cur` by madd).
pub fn mix(cur: ([f32; 4], [f32; 3]), other: ([f32; 4], [f32; 3]), w: f32) -> ([f32; 4], [f32; 3]) {
    use crate::ps2::{add, madd, sub};
    (crate::quat::slerp(cur.0, other.0, w), std::array::from_fn(|i| madd(add(0.0, cur.1[i]), sub(other.1[i], cur.1[i]), w)))
}

/// A stroke starting with `frames` to contact (+0x3ec4) on contact-search branch `branch` (+0x3ec1: 1 ground,
/// 2 volley, 3 dive, 4 smash) with swing motion `anim`. Returns the motion to play now, its speed, and the swing
/// still to come (played at speed 1 from 8 frames before contact) when the body first turns: a ground stroke or
/// volley more than 8 frames out turns sideways (`tb_f` 1 for forehand-side swings, `tb_b` 2 otherwise).
pub fn stroke_start(branch: u8, frames: i32, anim: i32) -> (i32, f32, Option<i32>) {
    if (1..=2).contains(&branch) && 8 < frames {
        return (if anim & 1 == 0 { 1 } else { 2 }, 1.0, Some(anim));
    }
    let speed = if branch == 3 || frames < 1 { 1.0 } else { 8.0 / frames as f32 };
    (anim, speed, None)
}

/// Frames before contact at which a pending swing starts.
pub const SWING_LEAD: i32 = 8;

/// The miss motion for a swing that met nothing (`None`: the swing has none).
pub fn whiff(anim: i32) -> Option<i32> {
    match anim {
        0x10 | 0x12 | 0x14 | 0x16 | 0x18 => Some(0x27),
        0x11 | 0x13 | 0x15 | 0x17 | 0x19 => Some(0x28),
        0x1f | 0x25 => Some(0x29),
        0x26 => Some(0x2a),
        _ => None,
    }
}

/// A whiff reaches its contact pose this many frames after the press (the swing's 8th motion frame).
pub const WHIFF_POSE: u32 = 9;
/// Frames after the pose before a stick, a press or the motion's end frees the player.
pub const WHIFF_RECOVERY: u32 = 30;

/// A swing at nothing, counted from the press. `miss`: there was a ball to hit, so at the pose the swing turns
/// into its miss motion (with a shout unless `quiet`) and a new press is taken from a short lock after it (2
/// frames, 30 after a smash 0x1f); with no ball for the player (`!miss`) the swing just plays on until its
/// recovery is over.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Whiff {
    pub anim: i32,
    pub miss: bool,
    pub quiet: bool,
    /// Frames since the press.
    pub tick: u32,
}

impl Whiff {
    pub fn new(anim: i32, miss: bool, quiet: bool) -> Self {
        Whiff { anim, miss, quiet, tick: 0 }
    }

    /// Frames since the contact pose, 0 on the pose frame.
    pub fn since(&self) -> Option<u32> {
        self.tick.checked_sub(WHIFF_POSE)
    }

    /// One frame on: the miss motion to switch to, on the pose frame.
    pub fn step(&mut self) -> Option<i32> {
        self.tick += 1;
        if self.tick == WHIFF_POSE && self.miss { whiff(self.anim) } else { None }
    }

    /// The re-press lock after the miss motion (`None`: no miss motion, so none).
    pub fn lock(&self) -> Option<u32> {
        (self.miss && whiff(self.anim).is_some()).then_some(if self.anim == 0x1f { 30 } else { 2 })
    }

    /// Whether a press this frame swings again, and if so whether quietly: through the re-press lock (no shout),
    /// else once the recovery is over.
    pub fn press(&self) -> Option<bool> {
        let s = self.since()?;
        if self.lock().is_some_and(|l| s >= l) {
            Some(true)
        } else {
            (s >= WHIFF_RECOVERY).then_some(false)
        }
    }

    /// Whether the whiff is over this frame without a press: recovered, and the stick moves or the motion has
    /// played out.
    pub fn over(&self, stick: bool, played: bool) -> bool {
        self.since().is_some_and(|s| s >= WHIFF_RECOVERY) && (stick || played)
    }
}

/// After a ground stroke's contact (branch 1, swings 0x10..0x15) a slow ball (|v|² < 0.2143347) switches the
/// next frame to the soft follow-through, `f_w` 0x1c or `b_w` 0x1d (crossfaded in, `SOFT_FOLLOW_HOLD`). `side` is the contact's
/// side bits (+0x3f50; bit 1 the left side), mirrored for left-handers.
pub fn soft_follow(branch: u8, anim: i32, ball_vel: [f32; 3], side: u32, hand: f32) -> Option<i32> {
    let v2 = ball_vel[2] * ball_vel[2] + ball_vel[0] * ball_vel[0] + ball_vel[1] * ball_vel[1];
    if !(v2 < 0.2143347 && (0x10..0x16).contains(&anim) && branch == 1) {
        return None;
    }
    let fore = (side & 2 == 0) != (hand < 0.0);
    Some(if fore { 0x1c } else { 0x1d })
}

/// The soft follow-through comes in over an 8-frame crossfade: its clock holds at 0 for this countdown.
pub const SOFT_FOLLOW_HOLD: i32 = 7;

/// Frames after contact before a stick or a shot press may break off the follow-through: 15 after a slice (kind 1)
/// off a ground stroke or volley (`branch` 1/2), else 30.
pub fn recovery(branch: u8, kind: i32) -> u32 {
    if kind == 1 && (branch == 1 || branch == 2) { 15 } else { 30 }
}

/// Whether a stroke's follow-through, `after` frames past contact (1 the frame after), hands off this frame to
/// standing or running: its motion `played` to the end (as of the last tick), or with `input` (stick or press) past
/// `recover` (the first chance comes two frames after it).
pub fn follow_over(after: u32, recover: u32, played: bool, input: bool) -> bool {
    played || (input && after >= recover + 2)
}

/// Serve: stance 0x20, walking the baseline (`serve_r` 0x22 when the stick points along the player's forward ×
/// x, `serve_l` 0x21 otherwise, swapped for left-handers), the toss (0x24 underhand, else 0x23) and the swing
/// (0x26 underhand, else 0x25, at speed 8 / frames to contact).
pub fn serve_walk(stick_x: f32, forward: f32, hand: f32) -> i32 {
    let right = 0.0 < stick_x * forward;
    if right != (hand < 0.0) { 0x22 } else { 0x21 }
}

pub fn serve_toss(under: bool) -> i32 {
    if under { 0x24 } else { 0x23 }
}

pub fn serve_swing(under: bool, frames: i32) -> (i32, f32) {
    (if under { 0x26 } else { 0x25 }, if frames < 1 { 1.0 } else { 8.0 / frames as f32 })
}

/// The point's reaction for a player: hit by the ball (`re_ball` 0x2b); one-player modes `di_set` 0x2f /
/// `gu_set` 0x2e; otherwise the winners `gu` 0x2c and the losers `di` 0x2d, or `gu_set` / `di_set` when the point
/// ends a game (`game`, 0x4230b8).
pub fn reaction(body_hit: bool, players: i32, won: bool, game: bool, practice_short: bool) -> i32 {
    if body_hit {
        0x2b
    } else if players < 2 {
        if practice_short { 0x2f } else { 0x2e }
    } else {
        match (won, game) {
            (true, false) => 0x2c,
            (true, true) => 0x2e,
            (false, false) => 0x2d,
            (false, true) => 0x2f,
        }
    }
}

/// Doubles team reactions (`re_pc00_co01_f` … `co05`, motions 0x30..0x34) a character may play instead of
/// `gu`/`di` (0x3fc790): characters 5, 7 and 11 have their own sets, the rest by parity.
pub fn team_reactions(character: i32) -> &'static [i32] {
    match character {
        5 => &[0, 1, 3, 4],
        7 | 11 => &[3, 4],
        c if c & 1 == 0 => &[0, 1, 2, 3, 4],
        _ => &[2, 3, 4],
    }
}

/// The team reaction drawn for `gu`/`di` in doubles: the character's set minus those taken by players updated
/// earlier this point; `draw` (the game's `(rand >> 16 & 0x7fff) % (n + 1)`) picks one, or keeps `base` when it
/// lands past the set.
pub fn team_reaction(base: i32, character: i32, taken: &[i32], draw: impl FnOnce(u32) -> u32) -> i32 {
    if !(0x2c..=0x2d).contains(&base) {
        return base;
    }
    let free: Vec<i32> = team_reactions(character).iter().copied().filter(|c| !taken.contains(c)).collect();
    let k = draw(free.len() as u32 + 1) as usize;
    free.get(k).map_or(base, |c| c + 0x30)
}

/// One frame of a post-point reaction's root motion: the reaction's path point `p` (its `*_dummy` motion at the
/// motion's time; zero without one) turned by the player's rows (right × hand, up, forward) and added to the
/// spot where the reaction started (`base`, +0x3d90). Returns the new accumulated spot (+0x3da0, from `acc`);
/// the player moves by the change. Team reactions (`team`, motions 0x30..) only go forward (path x, y dropped),
/// 0.55 as far for character 5; `gu_set` follows the whole path.
pub fn reaction_root(p: [f32; 4], team: bool, character: i32, rows: [[f32; 4]; 3], base: [f32; 4], acc: [f32; 4]) -> [f32; 4] {
    use crate::ps2::{add, madd, mul, sub};
    let m = [rows[0], rows[1], rows[2], [0.0, 0.0, 0.0, 1.0]];
    let to = if team {
        let v = crate::vu0::transform(&m, [0.0, 0.0, p[2], p[3]]);
        let s = if character == 5 { 0.55 } else { 1.0 };
        [add(base[0], mul(v[0], s)), madd(add(0.0, base[1]), v[1], s), madd(add(0.0, base[2]), v[2], s), madd(add(0.0, base[3]), v[3], s)]
    } else {
        let v = crate::vu0::transform(&m, p);
        std::array::from_fn(|k| add(base[k], v[k]))
    };
    std::array::from_fn(|k| add(acc[k], sub(to[k], acc[k])))
}
