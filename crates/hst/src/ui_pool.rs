//! P39: the match's overlays (pause menu and its Settings, result screen, online notice) draw into pools of image
//! quads. Bevy's UI systems (layout, stack, clipping, focus) walk every node every frame, hidden or not, so a pool is
//! only spawned while its screen is up and despawned when it goes: a match with none of them open has no pool nodes.

use bevy::prelude::*;

/// One quad of a pool: an absolutely placed, stretched image, hidden until drawn.
pub fn quad() -> impl Bundle {
    (ImageNode { image_mode: NodeImageMode::Stretch, ..default() }, Node { position_type: PositionType::Absolute, ..default() }, Visibility::Hidden)
}

/// Spawns `n` slots (`slot(i)`, `i` in order, so later ones draw on top) under `root`, before its other children (so
/// those still draw over them), when `on` and it has none,
/// despawns them when it's off. `have` is the pool's slots now. Run it chained before the draw, so the commands land
/// before it reads the slots: the screen's first frame is drawn.
pub fn keep<B: Bundle>(commands: &mut Commands, root: Entity, on: bool, have: impl IntoIterator<Item = Entity>, n: usize, slot: impl Fn(usize) -> B) {
    let mut have = have.into_iter().peekable();
    if on && have.peek().is_none() {
        let slots: Vec<Entity> = (0..n).map(|i| commands.spawn(slot(i)).id()).collect();
        commands.entity(root).insert_children(0, &slots);
    } else if !on {
        for e in have {
            commands.entity(e).despawn();
        }
    }
}

/// Sets a quad's image, source rect and tint, touching it (and so the frame's UI work) only if one changed.
pub fn paint(img: &mut Mut<ImageNode>, image: Handle<Image>, rect: Option<Rect>, color: Color) {
    if img.image != image || img.rect != rect || img.color != color {
        let img = &mut **img;
        (img.image, img.rect, img.color) = (image, rect, color);
    }
}
