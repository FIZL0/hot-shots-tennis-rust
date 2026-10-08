//! The select cards' 3D previews (B40b1): each card a player is picking on shows its hovered character in its costume
//! as the original's inspect screen draws it (`inspect.rs`), in place of the portrait, under the card's text; a new
//! hover or costume respawns it. That player's R2 / L2 / △ (keyboard: 1, 2 and the attributes key) play the
//! inspect screen's win, loss and back.
//!
//! The cards stack by quad order: every slot's `ZIndex` is its index in the layout, the preview's its `Tex::Preview`'s.
//! `HST_SELECT=c,c,c,c` (with `HST_MENU=chars`) seats all four players on the keyboard hovering those characters, to
//! `--shot` every card at once.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use hst_data::iso::Iso;

use super::{Art, Dev, Menu, Screen, Slot, Tex, layout};
use crate::character::{self, CharacterData, Motion};
use crate::inspect::{self, Act, Preview, PreviewMaterial};
use crate::Args;
use crate::play::controls::{Action, Bindings};

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(inspect::plugin)
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_systems(PostStartup, hover_all)
        .add_systems(FixedUpdate, character::tick)
        .add_systems(Update, ((sync, buttons).after(super::draw), character::animate));
}

/// A card's preview: what it shows, its camera and its UI node.
struct Shown {
    key: (usize, usize),
    cam: Entity,
    node: Entity,
}

/// Loaded characters by (character, costume), with their hand.
type Cache = HashMap<(usize, usize), Option<(Arc<CharacterData>, f32)>>;

#[allow(clippy::too_many_arguments)]
fn sync(
    mut commands: Commands,
    menu: Res<Menu>,
    art: Option<Res<Art>>,
    bind: Res<Bindings>,
    args: Res<Args>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut slots: Query<(&Slot, &ChildOf, &mut ZIndex), Without<Shot>>,
    mut nodes: Query<(&mut Node, &mut ZIndex), With<Shot>>,
    cams: Query<&Preview>,
    parents: Query<&ChildOf, Without<Slot>>,
    mut assets: (ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>, ResMut<Assets<Image>>, ResMut<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>),
    mut ui: ResMut<Assets<PreviewMaterial>>,
    mut shown: Local<[Option<Shown>; 4]>,
    mut cache: Local<Cache>,
    mut iso: Local<Option<Iso>>,
) {
    let Some(art) = art else { return };
    let quads = if menu.screen == Screen::Chars { layout(&menu, &art.2, &bind, &[], 0.0) } else { Vec::new() };
    let mut root = None;
    for (Slot(i), parent, mut z) in &mut slots {
        root = Some(parent.parent());
        if z.0 != *i as i32 {
            z.0 = *i as i32;
        }
    }
    let Some(root) = root else { return };
    let scale = window.single().map_or(1.0, |w| w.physical_height() as f32 / 448.0);
    for (p, slot) in shown.iter_mut().enumerate() {
        let want = quads.iter().enumerate().find(|(_, q)| q.tex == Tex::Preview(p));
        let key = want.map(|_| (menu.players[p].char(), menu.players[p].costume));
        if slot.as_ref().map(|s| s.key) != key {
            if let Some(s) = slot.take() {
                // the rig's parent is the preview's game space
                if let Ok(rig) = cams.get(s.cam).map(|c| c.rig)
                    && let Ok(space) = parents.get(rig)
                {
                    commands.entity(space.parent()).despawn();
                }
                commands.entity(s.cam).despawn();
                commands.entity(s.node).despawn();
            }
            if let (Some((c, costume)), Some((_, q))) = (key, want) {
                let iso = iso.get_or_insert_with(|| Iso::open(&args.iso).expect("open iso"));
                let (meshes, materials, images, binds) = &mut assets;
                let loaded = cache.entry((c, costume)).or_insert_with(|| match character::load_disc(iso, c, costume, meshes, materials, images, binds) {
                    Ok(d) => Some((Arc::new(d), inspect::disc_hand(iso, c))),
                    Err(e) => {
                        warn!("select preview of character {c} costume {costume}: {e}");
                        None
                    }
                });
                if let Some((data, hand)) = loaded.clone() {
                    let image = inspect::target(images, (Vec2::new(q.dst[2], q.dst[3]) * scale).as_uvec2());
                    let cam = inspect::spawn_into(&mut commands, &data, c, hand, 1 + p, image.clone());
                    commands.entity(cam).insert(Card(p));
                    let node = commands.spawn((Shot, Node { position_type: PositionType::Absolute, ..default() }, MaterialNode(ui.add(PreviewMaterial { image })))).id();
                    commands.entity(root).add_child(node);
                    *slot = Some(Shown { key: (c, costume), cam, node });
                }
            }
        }
        if let (Some(s), Some((i, q))) = (slot.as_ref(), want)
            && let Ok((mut node, mut z)) = nodes.get_mut(s.node)
        {
            let [x, y, w, h] = q.dst;
            node.left = Val::Percent(x / 6.4);
            node.top = Val::Percent(y / 4.48);
            node.width = Val::Percent(w / 6.4);
            node.height = Val::Percent(h / 4.48);
            z.0 = i as i32;
        }
    }
}

/// A preview's UI node.
#[derive(Component)]
struct Shot;

/// A preview camera's player.
#[derive(Component)]
struct Card(usize);

/// Each seated player's R2 / L2 / △ on their own preview.
fn buttons(
    menu: Res<Menu>,
    keys: Res<ButtonInput<KeyCode>>,
    bind: Res<Bindings>,
    pads: Query<&Gamepad>,
    cams: Query<(&Preview, &Card)>,
    data: Query<&character::Rig>,
    mut rigs: Query<(&mut Motion, &mut Transform)>,
) {
    if menu.screen != Screen::Chars {
        return;
    }
    for (p, Card(player)) in &cams {
        let Some(dev) = menu.seats[*player] else { continue };
        let pressed = |b: GamepadButton, k: bool| match dev {
            Dev::Keys => k,
            Dev::Pad(e) => pads.get(e).is_ok_and(|g| g.just_pressed(b)),
        };
        let act = if pressed(GamepadButton::RightTrigger2, keys.just_pressed(KeyCode::Digit1)) {
            Act::Win
        } else if pressed(GamepadButton::LeftTrigger2, keys.just_pressed(KeyCode::Digit2)) {
            Act::Loss
        } else if pressed(GamepadButton::North, bind.key_just_pressed(&keys, Action::Lob)) {
            Act::Pose
        } else {
            continue;
        };
        let Ok(rig) = data.get(p.rig) else { continue };
        let d = rig.data.clone();
        inspect::apply(p, act, &mut rigs, |id| d.motions.get(&id).map_or(0.0, |c| c.length));
    }
}

/// `HST_SELECT=c,c,c,c`: every player seated on the keyboard, hovering those characters (grid order is not
/// character order: each is looked up).
fn hover_all(mut menu: ResMut<Menu>) {
    let Ok(list) = std::env::var("HST_SELECT") else { return };
    for (p, c) in list.split(',').filter_map(|c| c.trim().parse::<usize>().ok()).take(4).enumerate() {
        if let Some(cursor) = super::GRID.iter().position(|g| g.0 == c) {
            menu.players[p].cursor = cursor;
            menu.seats[p] = Some(Dev::Keys);
        }
    }
}
