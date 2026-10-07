//! Court hole animation ([`hst_sim::court_anim`]) on the hole's GS materials, a frame per 60 Hz tick.

use bevy::prelude::*;

use crate::gs::GsMaterial;

#[derive(Component)]
pub struct Playing {
    pub anim: hst_sim::court_anim::CourtAnim,
    /// Each drawn material with the (material, packet) its offset and alpha come from.
    pub parts: Vec<(Handle<GsMaterial>, (usize, usize))>,
}

pub fn plugin(app: &mut App) {
    app.insert_resource(Time::<Fixed>::from_hz(60.0)).add_systems(FixedUpdate, tick);
}

fn tick(mut playing: Query<&mut Playing>, mut materials: ResMut<Assets<GsMaterial>>) {
    for mut p in &mut playing {
        let p = &mut *p;
        p.anim.tick();
        for (h, (m, pk)) in &p.parts {
            let Some(mut g) = materials.get_mut(h) else { continue };
            g.uniform.uv_offset = Vec2::from(p.anim.uv_offset(*m, *pk));
            g.uniform.color.w = p.anim.alphas[*m];
        }
    }
}
