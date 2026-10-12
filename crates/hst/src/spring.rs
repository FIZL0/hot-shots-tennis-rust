//! Spring bones (character standard §3b), mods only: a joint with `extras.spring` is a point mass hung on its
//! parent joint. Each drawn frame, after the pose, the mass trails where the pose puts it (pulled back by
//! `stiffness`, velocity kept by `damping`, sagging by `gravity`, at most `limit` degrees off the posed direction)
//! and its parent turns to point at it; a chain's first record (whose parent has none) is its pinned anchor. Out of Bounds' chains carry these (its `.rem` dynamics records); the
//! game's own solver is not ported (notes/oob.md *Dynamics*), so this is a plain mass-spring that looks the part.
//! Stock characters have no spring joints: their pose and the sim are untouched.

use bevy::prelude::*;

use crate::character::{Joint, Rig};

/// A spring joint: the joint, its parent (the one that turns) and its §3b parameters.
pub struct Mass {
    joint: usize,
    pub(crate) parent: usize,
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
    let mut out: Vec<(usize, Mass)> = nodes
        .iter()
        .enumerate()
        .filter_map(|(k, n)| {
            let e: serde_json::Value = serde_json::from_str(n.extras().as_ref()?.get()).ok()?;
            let s = &e["spring"];
            let f = |key: &str, default: f64| s[key].as_f64().unwrap_or(default) as f32;
            s.is_object().then_some(())?;
            Some((depth(k), Mass { joint: k, parent: joints[k].parent?, limit: f("limit", 30.0).to_radians(), stiffness: f("stiffness", 0.5), gravity: f("gravity", 4.9), damping: f("damping", 0.83) }))
        })
        .collect();
    // a chain's first record is its anchor (stiffness 1, limit 0..5°): pinned, it never turns the head it hangs on
    let springy: Vec<usize> = out.iter().map(|(_, m)| m.joint).collect();
    out.retain(|(_, m)| springy.contains(&m.parent));
    out.sort_by_key(|(d, _)| *d);
    out.into_iter().map(|(_, m)| m).collect()
}

/// A rig's masses in world space: position and last step's motion; `None` until the first pose.
#[derive(Component, Default)]
pub struct Spring {
    state: Vec<Option<(Vec3, Vec3)>>,
    clock: f32,
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

/// One 60 Hz step of a mass toward `target`, hung at `origin`; returns the new position and motion.
fn step(m: &Mass, origin: Vec3, target: Vec3, (x, v): (Vec3, Vec3)) -> (Vec3, Vec3) {
    let len = (target - origin).length();
    let v = v * m.damping + (target - x) * m.stiffness + Vec3::NEG_Y * m.gravity / 3600.0;
    let posed = (target - origin).normalize_or_zero();
    // only the motion across the segment turns it: a pull along it (gravity on a tip posed upright) would shorten
    // the step and the renormalised turn would overshoot, swinging for ever
    let r = (x - origin).normalize_or(posed);
    // ponytail: at most half the segment a step, so a whipped short one (an earring's 8 mm tip) can't flip through
    // its anchor; a sub-stepped solver if fast swings ever look stiff
    let v = (v - r * v.dot(r)).clamp_length_max(len * 0.5);
    let mut dir = (x + v - origin).normalize_or(posed);
    let off = posed.angle_between(dir);
    if off > m.limit {
        dir = Quat::from_rotation_arc(posed, dir).slerp(Quat::IDENTITY, 1.0 - m.limit / off) * posed;
    }
    let y = origin + dir * len;
    (y, y - x)
}

pub(crate) fn swing(time: Res<Time>, mut rigs: Query<(&Rig, &GlobalTransform, &mut Spring)>, mut joints: Query<&mut Transform>) {
    for (rig, global, mut s) in &mut rigs {
        let masses = springs(rig);
        s.state.resize(masses.len(), None);
        // ponytail: whole 60 Hz steps against this frame's pose (none while paused); sub-frame targets if it judders
        s.clock += time.delta_secs() * 60.0;
        let steps = (s.clock.floor() as usize).min(4);
        s.clock = (s.clock - s.clock.floor()).min(1.0);
        let mut world: Vec<Option<Mat4>> = vec![None; rig.joints.len()];
        fn posed(j: usize, rig: &Rig, root: Mat4, joints: &Query<&mut Transform>, w: &mut [Option<Mat4>]) -> Mat4 {
            if let Some(m) = w[j] {
                return m;
            }
            let local = joints.get(rig.joints[j]).map_or(Mat4::IDENTITY, |t| t.to_matrix());
            let m = rig.data.joints[j].parent.map_or(root, |p| posed(p, rig, root, joints, w)) * local;
            w[j] = Some(m);
            m
        }
        let root = global.to_matrix();
        for (m, state) in masses.iter().zip(s.state.iter_mut()) {
            let parent = posed(m.parent, rig, root, &joints, &mut world);
            let target = posed(m.joint, rig, root, &joints, &mut world).w_axis.truncate();
            let origin = parent.w_axis.truncate();
            // a fresh rig, or one placed somewhere new (a point's set-up), starts at the pose
            let mut st = state.filter(|(x, _)| x.distance(target) < 1.0).unwrap_or((target, Vec3::ZERO));
            for _ in 0..steps {
                st = step(m, origin, target, st);
            }
            *state = Some(st);
            let turn = Quat::from_rotation_arc((target - origin).normalize_or_zero(), (st.0 - origin).normalize_or_zero());
            if turn.is_nan() || turn.angle_between(Quat::IDENTITY) < 1e-6 {
                continue;
            }
            // the parent's world rotation turned by `turn`, back in its parent's frame
            let above = rig.data.joints[m.parent].parent.map_or(root, |p| posed(p, rig, root, &joints, &mut world));
            let (_, up, _) = above.to_scale_rotation_translation();
            let Ok(mut tf) = joints.get_mut(rig.joints[m.parent]) else { continue };
            tf.rotation = (up.inverse() * turn * up * tf.rotation).normalize();
            world[m.parent] = Some(above * tf.to_matrix());
            world[m.joint] = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mass(limit: f32) -> Mass {
        Mass { joint: 1, parent: 0, limit: limit.to_radians(), stiffness: 0.5, gravity: 4.9, damping: 0.83 }
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
    fn zero_limit_is_rigid() {
        let m = mass(0.0);
        let st = step(&m, Vec3::ZERO, Vec3::X, (Vec3::NEG_Y, Vec3::ZERO));
        assert!(st.0.distance(Vec3::X) < 1e-5);
    }
}
