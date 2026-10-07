//! The motion numbers the game sets outside standing and running (`player`): strokes (wind-up turn, the swing
//! timed to contact, the soft follow-through, whiffs), the serve, and the reactions after a point.

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

/// After a ground stroke's contact (branch 1, swings 0x10..0x15) a slow ball (|v|² < 0.2143347) switches the
/// next frame to the soft follow-through, `f_w` 0x1c or `b_w` 0x1d, from its frame 8. `side` is the contact's
/// side bits (+0x3f50; bit 1 the left side), mirrored for left-handers.
pub fn soft_follow(branch: u8, anim: i32, ball_vel: [f32; 3], side: u32, hand: f32) -> Option<i32> {
    let v2 = ball_vel[2] * ball_vel[2] + ball_vel[0] * ball_vel[0] + ball_vel[1] * ball_vel[1];
    if !(v2 < 0.2143347 && (0x10..0x16).contains(&anim) && branch == 1) {
        return None;
    }
    let fore = (side & 2 == 0) != (hand < 0.0);
    Some(if fore { 0x1c } else { 0x1d })
}

/// Start frame of the soft follow-through.
pub const SOFT_FOLLOW_FROM: f32 = 8.0;

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
