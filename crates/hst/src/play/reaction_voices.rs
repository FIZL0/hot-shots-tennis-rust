//! The reaction voices after a point (`hst_sim::sound::reaction_voice`): on the reactions' 5th frame each player
//! the post-point camera has in close view (`sound::reaction_view`) cheers or groans by their reaction motion.

use bevy::prelude::*;
use hst_sim::sound;

use super::Game;

pub fn plugin(app: &mut App) {
    app.add_systems(FixedUpdate, step.after(super::simulate).before(super::start_effects));
}

/// `frames`: frames since the reactions started.
fn step(mut g: ResMut<Game>, mut frames: Local<u32>) {
    if !g.post.as_ref().is_some_and(|p| p.reacted) {
        return *frames = 0;
    }
    *frames += 1;
    if *frames != 5 || g.players.len() < 2 {
        return;
    }
    let g = &mut *g;
    let v = g.cam.view;
    let [x, y, z] = v.eye;
    let eye = [x, y, z, 1.0];
    let [x, y, z] = v.rot[2];
    let look = [x, y, z, 0.0];
    let view = matrix(&v);
    let focal = 240.0 / v.fov.tan();
    let n = g.players.len() as u32;
    for i in 0..g.players.len() {
        let p = &g.players[i];
        // ponytail: the player's position stands for both the model origin and the spot the game tests
        let at = [p.pos[0], p.pos[1], p.pos[2], 1.0];
        if !sound::reaction_view(eye, look, &view, focal, at, at) {
            continue;
        }
        // ponytail: a body-hit player standing has no reaction motion; any other motion voices alike
        let motion = p.root.map_or(-1, |r| r.motion as i32);
        let lost = i as i32 & 1 != g.post_winner & 1;
        let Some(rv) = sound::reaction_voice(n, motion, lost, false, || g.rng.shared.r15() % 100) else { continue };
        if let Some(play) = g.voices[i].react(i, rv, || g.rng.shared.r15()) {
            g.whooshes.push((0, i, play));
        }
    }
}

/// `v`'s world → camera matrix as `vu0::transform` takes it (column k is camera axis k).
fn matrix(v: &hst_sim::camera::View) -> [[f32; 4]; 4] {
    let mut m = [[0.0; 4]; 4];
    for k in 0..3 {
        for j in 0..3 {
            m[j][k] = v.rot[k][j];
        }
        m[3][k] = -(0..3).map(|j| v.rot[k][j] * v.eye[j]).sum::<f32>();
    }
    m[3][3] = 1.0;
    m
}

#[cfg(test)]
mod tests {
    #[test]
    fn matrix_is_local() {
        let v = hst_sim::camera::View { rot: [[0.6, 0.0, -0.8], [0.0, 1.0, 0.0], [0.8, 0.0, 0.6]], eye: [1.0, -2.0, 3.0], fov: 0.5 };
        let p = [4.0, 5.0, -6.0];
        let got = hst_sim::vu0::transform(&super::matrix(&v), [p[0], p[1], p[2], 1.0]);
        let want = v.local(p);
        assert!((0..3).all(|k| (got[k] - want[k]).abs() < 1e-5), "{got:?} {want:?}");
    }
}
