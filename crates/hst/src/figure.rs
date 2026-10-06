//! Original stand-in athlete: an articulated primitive figure with procedural animation (idle, run cycle,
//! forehand/backhand swing, serve). Built in game space (Y-down, feet at the origin, facing local -z).

use bevy::prelude::*;

/// Animation inputs written by the gameplay code each frame.
#[derive(Component, Default, Clone, Copy)]
pub struct Pose {
    /// Ground speed in metres per frame; drives the run cycle.
    pub speed: f32,
    /// Accumulated stride phase (radians).
    pub stride: f32,
    /// 0..1 progress through a swing, `None` when not swinging.
    pub swing: Option<f32>,
    pub backhand: bool,
    pub serving: bool,
}

#[derive(Component, Clone, Copy)]
pub enum Joint {
    Hips,
    Torso,
    Head,
    ArmR,
    ArmL,
    LegR,
    LegL,
}

/// Spawn a figure under `parent`; returns the figure's root entity (carry `Pose` on it).
pub fn spawn(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, shirt: Color, parent: Entity) -> Entity {
    let skin = materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.66, 0.5), perceptual_roughness: 0.8, ..default() });
    let top = materials.add(StandardMaterial { base_color: shirt, perceptual_roughness: 0.7, ..default() });
    let shorts = materials.add(StandardMaterial { base_color: Color::srgb(0.95, 0.95, 0.95), perceptual_roughness: 0.7, ..default() });
    let frame = materials.add(StandardMaterial { base_color: Color::srgb(0.15, 0.15, 0.18), metallic: 0.6, ..default() });
    let strings = materials.add(StandardMaterial { base_color: Color::srgba(0.9, 0.9, 0.85, 0.6), alpha_mode: AlphaMode::Blend, ..default() });
    let limb = meshes.add(Capsule3d::new(0.06, 0.62));
    let leg = meshes.add(Capsule3d::new(0.075, 0.74));
    let torso = meshes.add(Capsule3d::new(0.17, 0.42));
    let pelvis = meshes.add(Capsule3d::new(0.16, 0.1));
    let head = meshes.add(Sphere::new(0.12));
    let handle = meshes.add(Cylinder::new(0.016, 0.3));
    let face = meshes.add(Cylinder::new(0.13, 0.02));

    let root = commands.spawn((Pose::default(), Transform::default(), Visibility::default())).id();
    commands.entity(parent).add_child(root);
    // y is down in game space: hips at -0.95 m, shoulders ~0.5 m above the hips
    let hips = commands
        .spawn((Joint::Hips, Transform::from_xyz(0.0, -0.95, 0.0), Visibility::default()))
        .with_child((Mesh3d(pelvis), MeshMaterial3d(shorts.clone()), Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))))
        .id();
    commands.entity(root).add_child(hips);
    for (j, x) in [(Joint::LegR, 0.1), (Joint::LegL, -0.1)] {
        let l = commands
            .spawn((j, Transform::from_xyz(x, 0.05, 0.0), Visibility::default()))
            .with_child((Mesh3d(leg.clone()), MeshMaterial3d(skin.clone()), Transform::from_xyz(0.0, 0.45, 0.0)))
            .id();
        commands.entity(hips).add_child(l);
    }
    let torso_e = commands
        .spawn((Joint::Torso, Transform::from_xyz(0.0, -0.1, 0.0), Visibility::default()))
        .with_child((Mesh3d(torso), MeshMaterial3d(top.clone()), Transform::from_xyz(0.0, -0.28, 0.0)))
        .id();
    commands.entity(hips).add_child(torso_e);
    let head_e = commands.spawn((Joint::Head, Mesh3d(head), MeshMaterial3d(skin.clone()), Transform::from_xyz(0.0, -0.72, 0.0))).id();
    commands.entity(torso_e).add_child(head_e);
    for (j, x) in [(Joint::ArmR, 0.24), (Joint::ArmL, -0.24)] {
        let a = commands
            .spawn((j, Transform::from_xyz(x, -0.5, 0.0), Visibility::default()))
            .with_child((Mesh3d(limb.clone()), MeshMaterial3d(skin.clone()), Transform::from_xyz(0.0, 0.36, 0.0)))
            .id();
        if matches!(j, Joint::ArmR) {
            // racket in the right hand, pointing down the arm
            let r = commands
                .spawn((Transform::from_xyz(0.0, 0.72, 0.0), Visibility::default()))
                .with_child((Mesh3d(handle.clone()), MeshMaterial3d(frame.clone()), Transform::from_xyz(0.0, 0.15, 0.0)))
                .with_child((Mesh3d(face.clone()), MeshMaterial3d(strings.clone()), Transform::from_xyz(0.0, 0.42, 0.0).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))))
                .id();
            commands.entity(a).add_child(r);
        }
        commands.entity(torso_e).add_child(a);
    }
    root
}

/// Procedural pose from the `Pose` inputs. Angles are about the joint's local axes; game space is Y-down,
/// so a positive X rotation swings a limb forward (toward local -z).
pub fn animate(poses: Query<(&Pose, &Children)>, children: Query<&Children>, mut joints: Query<(&Joint, &mut Transform)>) {
    for (pose, kids) in &poses {
        let run = (pose.speed / 0.1).clamp(0.0, 1.2);
        let s = pose.stride.sin();
        let mut stack: Vec<Entity> = kids.iter().collect();
        while let Some(e) = stack.pop() {
            if let Ok(c) = children.get(e) {
                stack.extend(c.iter());
            }
            let Ok((joint, mut t)) = joints.get_mut(e) else { continue };
            let (swing_yaw, arm_r, arm_l) = swing_pose(pose);
            t.rotation = match joint {
                Joint::Hips => Quat::from_rotation_x(-0.15 * run),
                Joint::Torso => Quat::from_rotation_y(swing_yaw) * Quat::from_rotation_x(-0.1 * run + 0.03 * (pose.stride * 2.0).sin() * run),
                Joint::Head => Quat::from_rotation_y(-swing_yaw * 0.6),
                Joint::LegR => Quat::from_rotation_x(0.7 * s * run),
                Joint::LegL => Quat::from_rotation_x(-0.7 * s * run),
                Joint::ArmR => arm_r.unwrap_or(Quat::from_rotation_x(-0.6 * s * run) * Quat::from_rotation_z(0.25)),
                Joint::ArmL => arm_l.unwrap_or(Quat::from_rotation_x(0.6 * s * run) * Quat::from_rotation_z(-0.2)),
            };
        }
    }
}

/// Torso twist and arm rotations for the current swing/serve, if any.
fn swing_pose(p: &Pose) -> (f32, Option<Quat>, Option<Quat>) {
    let Some(t) = p.swing else {
        return (0.0, None, None);
    };
    let ease = |x: f32| x * x * (3.0 - 2.0 * x);
    if p.serving {
        // toss arm up, racket arm drops back then whips over the top
        let up = ease((t * 2.0).min(1.0));
        let whip = ease(((t - 0.35) / 0.65).clamp(0.0, 1.0));
        let arm_l = Quat::from_rotation_x(2.6 * up * (1.0 - whip));
        let arm_r = Quat::from_rotation_x(-0.8 + 3.8 * whip) * Quat::from_rotation_z(0.4 * (1.0 - whip));
        return (-0.6 + 1.2 * whip, Some(arm_r), Some(arm_l));
    }
    // backswing to t=0.4, contact ~0.45, follow-through to 1
    let side = if p.backhand { -1.0 } else { 1.0 };
    let phase = if t < 0.4 { -ease(t / 0.4) } else { -1.0 + 2.2 * ease(((t - 0.4) / 0.6).min(1.0)) };
    let yaw = side * 1.1 * phase;
    let arm_r = Quat::from_rotation_y(side * 1.3 * phase) * Quat::from_rotation_z(side * 1.35) * Quat::from_rotation_x(0.3);
    let arm_l = if p.backhand { Some(Quat::from_rotation_y(-1.1 * phase) * Quat::from_rotation_z(-1.2)) } else { None };
    (yaw, Some(arm_r), arm_l)
}
