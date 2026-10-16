//! Spring bones (character standard §3b), mods only: a joint with `extras.spring` swings about its own origin. Its
//! tip (its child in the chain, else its rest offset carried on) is a point mass: each drawn frame, after the pose,
//! the mass trails where the pose puts it (velocity kept by `damping`, sagging by `gravity`, drawn `stiffness` of
//! the way back to the pose each 60 Hz step, at most `limit` degrees off the posed direction) and the joint turns to point at it; a 0° record is
//! rigid. Out of Bounds' chains carry these (its `.rem` dynamics records: a chain's first record is the bone at the
//! head, an earring's swinging, a ponytail root's held by stiffness 1 and a 0–5° limit); the game's own solver is
//! not ported (notes/oob.md *Dynamics*), so this is a plain mass-spring that looks the part.
//! Stock characters have no spring joints: their pose and the sim are untouched.

use bevy::prelude::*;

use crate::character::{Joint, Rig};

/// Where a spring joint's mass sits: on a child joint, or at an offset in the joint's own frame (a chain's leaf).
#[derive(Clone, Copy)]
enum Tip {
    Child(usize),
    Offset(Vec3),
}

/// A spring joint: the joint (the one that turns), its tip and its §3b parameters.
pub struct Mass {
    pub(crate) joint: usize,
    tip: Tip,
    limit: f32,
    stiffness: f32,
    gravity: f32,
    damping: f32,
}

/// The skin's spring joints (`nodes` in skin order, as `joints`), parents before children.
pub fn masses(nodes: &[gltf::Node], joints: &[Joint]) -> Vec<Mass> {
    let depth = |mut j: usize| {
        let mut d = 0;
        while let Some(p) = joints[j].parent {
            j = p;
            d += 1;
        }
        d
    };
    let spring = |k: usize| spring_of(nodes, k);
    let springy: Vec<bool> = (0..joints.len()).map(|k| spring(k).is_some()).collect();
    let mut out: Vec<(usize, Mass)> = (0..joints.len())
        .filter_map(|k| {
            let s = spring(k)?;
            let f = |key: &str, default: f64| s[key].as_f64().unwrap_or(default) as f32;
            let limit = f("limit", 30.0).to_radians();
            // a 0° record (a ponytail's root) never turns; neither does a joint at the skeleton's top
            joints[k].parent?;
            (limit > 0.0).then_some(())?;
            // the tip: a child in the chain first (a pre-mirrored costume hangs its twin's chain beside its own), else
            // any child, else the joint's rest offset from its parent carried on in its own frame
            let children: Vec<usize> = (0..joints.len()).filter(|&c| joints[c].parent == Some(k)).collect();
            let rest = joints[k].rest;
            let tip = children.iter().find(|&&c| springy[c]).or(children.first()).map_or(Tip::Offset(rest.rotation.inverse() * rest.translation), |&c| Tip::Child(c));
            Some((depth(k), Mass { joint: k, tip, limit, stiffness: f("stiffness", 0.5), gravity: f("gravity", 4.9), damping: f("damping", 0.83) }))
        })
        .collect();
    out.sort_by_key(|(d, _)| *d);
    out.into_iter().map(|(_, m)| m).collect()
}

/// Joint `k`'s §3b `extras.spring`.
fn spring_of(nodes: &[gltf::Node], k: usize) -> Option<serde_json::Value> {
    let e: serde_json::Value = serde_json::from_str(nodes.get(k)?.extras().as_ref()?.get()).ok()?;
    e["spring"].is_object().then(|| e["spring"].clone())
}

/// The joints a rig's springs move: the spring joints and everything under them. A mod's own motions leave them
/// to the springs (Out of Bounds' clips key these bones in a space of their own, 60°+ jumps within a reaction).
pub fn moved(nodes: &[gltf::Node], joints: &[Joint]) -> Vec<bool> {
    let springy: Vec<bool> = (0..joints.len()).map(|k| spring_of(nodes, k).is_some()).collect();
    (0..joints.len())
        .map(|mut j| loop {
            if springy[j] {
                break true;
            }
            match joints[j].parent {
                Some(p) => j = p,
                None => break false,
            }
        })
        .collect()
}

/// A rig's masses: position and motion in world space, and the direction each pointed at the last two steps in its
/// joint's parent's frame (drawn between them); `None` until the first pose.
#[derive(Component, Default)]
pub struct Spring {
    state: Vec<Option<State>>,
    clock: f32,
}

#[derive(Clone, Copy)]
struct State {
    x: Vec3,
    v: Vec3,
    was: Vec3,
    now: Vec3,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (attach, swing.after(crate::character::animate).before(crate::noise::draw)));
}

fn springs(rig: &Rig) -> &[Mass] {
    rig.data.noise.as_ref().map_or(&[], |c| &c.springs)
}

fn attach(mut commands: Commands, rigs: Query<(Entity, &Rig), Added<Rig>>) {
    // HST_NO_SPRINGS=1: the rigid bind-pose hair, to compare against
    if std::env::var_os("HST_NO_SPRINGS").is_some() {
        return;
    }
    for (root, rig) in &rigs {
        if !springs(rig).is_empty() {
            commands.entity(root).try_insert(Spring::default());
        }
    }
}

/// One 60 Hz step of a mass toward `target`, hung at `origin`; returns the new position and motion. The motion
/// kept (`damping`) and gravity carry the mass on, then it is drawn `stiffness` of the way back to the pose: 1 holds
/// the pose (Out of Bounds' chain roots), 0.1 lets a tip sway.
fn step(m: &Mass, origin: Vec3, target: Vec3, (x, v): (Vec3, Vec3)) -> (Vec3, Vec3) {
    let len = (target - origin).length();
    let v = v * m.damping + Vec3::NEG_Y * m.gravity / 3600.0;
    let posed = (target - origin).normalize_or_zero();
    // only the motion across the segment turns it: a pull along it (gravity on a tip posed upright) would shorten
    // the step and the renormalised turn would overshoot, swinging for ever
    let r = (x - origin).normalize_or(posed);
    // ponytail: at most half the segment a step, so a whipped short one (an earring's 8 mm tip) can't flip through
    // its anchor; a sub-stepped solver if fast swings ever look stiff
    let v = (v - r * v.dot(r)).clamp_length_max(len * 0.5);
    let free = x + v;
    let mut dir = (free + (target - free) * m.stiffness - origin).normalize_or(posed);
    let off = posed.angle_between(dir);
    if off > m.limit {
        dir = Quat::from_rotation_arc(posed, dir).slerp(Quat::IDENTITY, 1.0 - m.limit / off) * posed;
    }
    let y = origin + dir * len;
    (y, y - x)
}

/// The local rotation of a joint under `above` (its parent's world matrix) whose world rotation is turned by `turn`.
fn turned(above: Mat4, turn: Quat, local: Quat) -> Quat {
    let a = Mat3::from_mat4(above);
    Quat::from_mat3(&(a.inverse() * Mat3::from_quat(turn) * a * Mat3::from_quat(local))).normalize()
}

pub(crate) fn swing(time: Res<Time>, mut rigs: Query<(&Rig, &GlobalTransform, &mut Spring, Has<crate::shadow::Minor>, Option<&Visibility>)>, mut joints: Query<&mut Transform>) {
    for (rig, global, mut s, minor, shown) in &mut rigs {
        // not posed while put away (`character::animate`), so not swung on top of the last pose either
        if crate::character::put_away(minor, shown) {
            continue;
        }
        let masses = springs(rig);
        s.state.resize(masses.len(), None);
        // whole 60 Hz steps against this frame's pose (none while paused), each mass hung on where the step before it
        // in the chain left its parent; then the frame draws each joint part way from the last step's direction to
        // this one's, held in its parent's frame so it moves with the body between steps
        s.clock += time.delta_secs() * 60.0;
        let steps = (s.clock.floor() as usize).min(4);
        s.clock = (s.clock - s.clock.floor()).min(1.0);
        let along = s.clock;
        let root = global.to_matrix();
        let pose: Vec<Quat> = masses.iter().map(|m| joints.get(rig.joints[m.joint]).map_or(Quat::IDENTITY, |t| t.rotation)).collect();
        let fresh = s.state.iter().all(Option::is_none);
        for pass in 0..steps.max(fresh as usize) + 1 {
            let draw = pass == steps.max(fresh as usize);
            let mut world: Vec<Option<Mat4>> = vec![None; rig.joints.len()];
            for (k, m) in masses.iter().enumerate() {
                if let Ok(mut tf) = joints.get_mut(rig.joints[m.joint]) {
                    tf.rotation = pose[k];
                }
            }
            for (k, m) in masses.iter().enumerate() {
                let above = rig.data.joints[m.joint].parent.map_or(root, |p| posed(p, rig, root, &joints, &mut world));
                let at = posed(m.joint, rig, root, &joints, &mut world);
                let origin = at.w_axis.truncate();
                let target = match m.tip {
                    Tip::Child(c) => posed(c, rig, root, &joints, &mut world).w_axis.truncate(),
                    Tip::Offset(o) => at.transform_point3(o),
                };
                let posed_dir = (target - origin).normalize_or_zero();
                let back = above.inverse();
                let local = |d: Vec3| back.transform_vector3(d).normalize_or_zero();
                let mut dir = if draw {
                    s.state[k].map_or(posed_dir, |st| above.transform_vector3(st.was.lerp(st.now, along)).normalize_or(posed_dir))
                } else {
                    // a fresh rig, or one placed somewhere new (a point's set-up), starts at the pose
                    let mut st = s.state[k].filter(|st| st.x.distance(target) < 1.0).unwrap_or(State { x: target, v: Vec3::ZERO, was: local(posed_dir), now: local(posed_dir) });
                    if pass < steps {
                        (st.x, st.v) = step(m, origin, target, (st.x, st.v));
                        (st.was, st.now) = (st.now, local(st.x - origin));
                    }
                    s.state[k] = Some(st);
                    (st.x - origin).normalize_or(posed_dir)
                };
                let off = posed_dir.angle_between(dir);
                if off > m.limit {
                    dir = Quat::from_rotation_arc(posed_dir, dir).slerp(Quat::IDENTITY, 1.0 - m.limit / off) * posed_dir;
                }
                let turn = Quat::from_rotation_arc(posed_dir, dir);
                if turn.is_nan() || turn.angle_between(Quat::IDENTITY) < 1e-6 {
                    continue;
                }
                // the joint's world rotation turned by `turn`, back in its parent's frame: through the whole matrix, as
                // a left-hander's root mirrors (scale x −1) and a decomposed rotation would turn the hair the wrong way
                let Ok(mut tf) = joints.get_mut(rig.joints[m.joint]) else { continue };
                tf.rotation = turned(above, turn, tf.rotation);
                world[m.joint] = Some(above * tf.to_matrix());
                if let Tip::Child(c) = m.tip {
                    world[c] = None;
                }
            }
        }
    }
}

/// Joint `j`'s world matrix under `root` as its transforms stand, cached in `w`.
fn posed(j: usize, rig: &Rig, root: Mat4, joints: &Query<&mut Transform>, w: &mut [Option<Mat4>]) -> Mat4 {
    if let Some(m) = w[j] {
        return m;
    }
    let local = joints.get(rig.joints[j]).map_or(Mat4::IDENTITY, |t| t.to_matrix());
    let m = rig.data.joints[j].parent.map_or(root, |p| posed(p, rig, root, joints, w)) * local;
    w[j] = Some(m);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mass(limit: f32) -> Mass {
        Mass { joint: 0, tip: Tip::Child(1), limit: limit.to_radians(), stiffness: 0.5, gravity: 4.9, damping: 0.83 }
    }

    #[test]
    fn trails_then_settles_within_the_limit() {
        let m = mass(30.0);
        let origin = Vec3::ZERO;
        // the head jerks sideways: the posed tip jumps, the mass lags, bounded by the limit, then settles back
        let mut st = (Vec3::NEG_Y * 0.2, Vec3::ZERO);
        let target = Vec3::new(0.2, 0.0, 0.0);
        st = step(&m, origin, target, st);
        let off = target.angle_between(st.0);
        assert!(off > 0.1 && off <= 30f32.to_radians() + 1e-4, "{off}");
        assert!((st.0.length() - 0.2).abs() < 1e-5);
        for _ in 0..600 {
            st = step(&m, origin, target, st);
        }
        assert!(st.0.distance(target) < 0.01, "{:?}", st.0);
    }

    #[test]
    fn a_turn_lands_under_a_mirrored_parent() {
        // B410: a left-hander's root has scale x −1; the turned joint's child must end up where the world turn puts it
        let local = Quat::from_rotation_z(0.4) * Quat::from_rotation_x(-0.2);
        let child = Vec3::new(0.0, -0.1, 0.02);
        let turn = Quat::from_rotation_arc(Vec3::NEG_Y, Vec3::new(0.3, -1.0, 0.2).normalize());
        for hand in [1.0, -1.0] {
            let above = Mat4::from_scale_rotation_translation(Vec3::new(hand, 1.0, 1.0), Quat::from_rotation_y(2.0) * Quat::from_rotation_x(0.3), Vec3::new(3.0, 1.0, 12.0));
            let tip = |q: Quat| above.transform_vector3(q * child);
            let want = turn * tip(local);
            let got = tip(turned(above, turn, local));
            assert!(got.distance(want) < 1e-5, "hand {hand}: {got} vs {want}");
        }
    }

    fn k_tip(m: &Mass) -> usize {
        match m.tip {
            Tip::Child(c) => c,
            Tip::Offset(_) => unreachable!(),
        }
    }

    /// A chain hung off a head moved by `head(frame)` (translation): per frame the masses step in order, each turning its
    /// own subtree as `swing` does; returns each frame's largest angle off the pose.
    fn chain(params: &[(f32, f32, f32)], seg: f32, frames: usize, head: impl Fn(usize) -> Vec3) -> Vec<f32> {
        let masses: Vec<Mass> = params.iter().enumerate().map(|(k, &(limit, stiffness, damping))| Mass { joint: k, tip: Tip::Child(k + 1), limit: limit.to_radians(), stiffness, gravity: 4.9, damping }).collect();
        // rest: hanging out sideways and down, a slight curve
        let rest: Vec<Vec3> = (0..=params.len()).map(|k| Vec3::new(k as f32 * seg * 0.7, -(k as f32) * seg * 0.7, 0.0)).collect();
        let mut state: Vec<Option<(Vec3, Vec3)>> = vec![None; masses.len()];
        let mut out = Vec::new();
        for f in 0..frames {
            let mut p: Vec<Vec3> = rest.iter().map(|r| *r + head(f)).collect();
            let posed = p.clone();
            let mut worst = 0f32;
            for (i, m) in masses.iter().enumerate() {
                let (origin, target) = (p[m.joint], p[k_tip(m)]);
                let st = step(m, origin, target, state[i].unwrap_or((target, Vec3::ZERO)));
                state[i] = Some(st);
                let turn = Quat::from_rotation_arc((target - origin).normalize(), (st.0 - origin).normalize());
                for q in p.iter_mut().skip(m.joint + 1) {
                    *q = origin + turn * (*q - origin);
                }
                worst = worst.max((p[k_tip(m)] - p[m.joint]).angle_between(posed[k_tip(m)] - posed[m.joint]));
            }
            out.push(worst);
        }
        out
    }

    #[test]
    fn a_still_chain_settles() {
        // Sophie's pigtail (L_kami01..10): limit, stiffness, damping
        let kami = [(5.0, 0.7, 0.83), (30.0, 0.5, 0.83), (20.0, 0.4, 0.73), (20.0, 0.3, 0.8), (20.0, 0.3, 0.83), (20.0, 0.5, 0.83), (20.0, 0.2, 0.83), (5.0, 0.7, 0.83), (0.0, 0.7, 0.83), (0.0, 0.7, 0.83)];
        // the head jumps 30 cm across, then holds still: the tail swings out, then comes to rest (sagging, each link from the one above)
        let off = chain(&kami, 0.04, 600, |f| if f < 10 { Vec3::ZERO } else { Vec3::X * 0.3 });
        assert!(off[11] > 20f32.to_radians(), "{}", off[11].to_degrees());
        assert!((off[599] - off[400]).abs() < 1e-3 && off[599] < 30f32.to_radians(), "{} {}", off[400].to_degrees(), off[599].to_degrees());
    }

    #[test]
    fn zero_limit_is_rigid() {
        let m = mass(0.0);
        let st = step(&m, Vec3::ZERO, Vec3::X, (Vec3::NEG_Y, Vec3::ZERO));
        assert!(st.0.distance(Vec3::X) < 1e-5);
    }
}
