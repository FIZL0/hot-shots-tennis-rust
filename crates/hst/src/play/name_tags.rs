//! B371: player names over the players in a match. A named player's "1P".."4P" marker (`markers`) is swapped for
//! a tag with the name in the player's panel colour, with a small pointer under it, hung at the same spot (the
//! pointer's tip on the marker's anchor) and as tall on screen as the marker. Unnamed players keep the original's
//! marker. The names come from the main menu's name picker (`--names a,,b`, by match slot); without any the match
//! looks as before. The point boards' plates, the final board and the result screen write the name where the slot
//! label was (`take`, `labels`). Drawing only: nothing here touches the sim.

use bevy::prelude::*;

use super::markers::{self, Marker};
use super::panel::Colours;
use super::widescreen;
use super::Game;

/// The players' names by match slot (empty: no name, the marker shows).
#[derive(Resource, Clone, Default, Debug, PartialEq)]
pub struct Names(pub Vec<String>);

impl Names {
    /// `a,,b` → ["a", "", "b"].
    pub fn parse(s: &str) -> Self {
        Self(s.split(',').map(|n| n.trim().to_string()).collect())
    }

    /// Player `i`'s name, if they have one.
    pub fn get(&self, i: usize) -> Option<&str> {
        self.0.get(i).map(String::as_str).filter(|n| !n.is_empty())
    }
}

/// One player's tag: the root node (a column centred on the anchor), its body, text and pointer.
#[derive(Component)]
struct Tag {
    i: usize,
    body: Entity,
    text: Entity,
    pointer: Entity,
}

/// The root's width (percent of the HUD's): room for any name, centred on the anchor.
const ROOM: f32 = 40.0;

pub fn plugin(app: &mut App) {
    app.init_resource::<Plates>()
        .add_systems(Update, (setup, draw).chain().after(markers::draw))
        // after every Update draw that fills `Plates`
        .add_systems(PostUpdate, labels.before(bevy::ui::UiSystems::Prepare));
}

/// A named player's slot label on a HUD this frame (640×448 rect, tint and alpha out of 128): `labels` writes the
/// name there instead.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Plate {
    player: usize,
    dst: [f32; 4],
    rgb: [f32; 3],
    alpha: f32,
}

/// A label's room for the name over its art's width.
const ROOM_K: f32 = 1.2;

/// Which HUD a pool of labels is on.
pub(super) const BOARDS: usize = 0;
pub(super) const RESULT: usize = 1;

/// Each HUD's plates this frame (`BOARDS`, `RESULT`).
#[derive(Resource, Default)]
pub(super) struct Plates(pub(super) [Vec<Plate>; 2]);

/// A name label in a HUD's pool: the HUD and its index.
#[derive(Component)]
struct Label(usize, usize);

/// Spawns `hud`'s labels under its 4:3 root, after its quads so they draw over them.
pub(super) fn pool(p: &mut bevy::ecs::hierarchy::ChildSpawnerCommands, hud: usize) {
    for k in 0..4 {
        p.spawn((Label(hud, k), Node { position_type: PositionType::Absolute, align_items: AlignItems::Center, ..default() }, Visibility::Hidden))
            .with_child((
                Text::new(""),
                TextFont::from_font_size(16.0),
                TextColor(Color::WHITE),
                TextShadow { offset: Vec2::splat(2.0), color: Color::srgba(0.0, 0.0, 0.0, 0.9) },
                TextLayout::no_wrap(),
            ));
    }
}

/// Takes the named players' slot labels out of `quads`: `label` gives a quad's slot if it is a slot label, `parts`
/// its rect, tint and alpha; `slots` is each match player's slot (humans 0..3, unique). The rest stay as they are.
pub(super) fn take<Q>(
    quads: &mut Vec<Q>,
    names: Option<&Names>,
    slots: &[usize],
    label: impl Fn(&Q) -> Option<usize>,
    parts: impl Fn(&Q) -> ([f32; 4], [f32; 3], f32),
) -> Vec<Plate> {
    let Some(names) = names else { return Vec::new() };
    let mut out = Vec::new();
    quads.retain(|q| {
        let player = label(q).filter(|&s| s < 4).and_then(|s| slots.iter().position(|&x| x == s)).filter(|&i| names.get(i).is_some());
        let Some(player) = player else { return true };
        let (dst, rgb, alpha) = parts(q);
        out.push(Plate { player, dst, rgb, alpha });
        false
    });
    out
}

/// Writes each HUD's plates' names where their labels were: from the label's left edge on the screen's left half,
/// to its right edge on the right half (where the face is on the other side), in the label's tint.
#[allow(clippy::type_complexity)]
fn labels(
    plates: Res<Plates>,
    names: Option<Res<Names>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut q: Query<(&Label, &mut Node, &mut Visibility, &Children)>,
    mut texts: Query<(&mut Text, &mut TextFont, &mut TextColor)>,
) {
    let hud = window.single().map_or(Vec2::new(640.0, 448.0), widescreen::view_size);
    // the 4:3 screen's height in HUD px (letterboxed in a window taller than 4:3)
    let px = hud.y.min(hud.x * 0.75) / 448.0;
    for (Label(h, k), mut node, mut vis, kids) in &mut q {
        let Some((p, name)) = plates.0[*h].get(*k).and_then(|p| Some((p, names.as_ref()?.get(p.player)?))) else {
            vis.set_if_neq(Visibility::Hidden);
            continue;
        };
        let [x, y, w, hh] = p.dst;
        let right = x + w / 2.0 > 320.0;
        // the label's room, a little past its art (the cell has padding); a long name shrinks to fit it (the
        // default font is monospace: 0.6 em a character)
        let room = w * ROOM_K;
        let size = (room / (0.62 * name.chars().count().max(1) as f32 + 0.3)).min(22.0);
        let placed = Node {
            left: if right { Val::Auto } else { Val::Percent(x / 6.4) },
            right: if right { Val::Percent((640.0 - x - w) / 6.4) } else { Val::Auto },
            top: Val::Percent(y / 4.48),
            width: Val::Percent(room / 6.4),
            height: Val::Percent(hh / 4.48),
            justify_content: if right { JustifyContent::End } else { JustifyContent::Start },
            overflow: Overflow::clip(),
            ..node.clone()
        };
        node.set_if_neq(placed);
        if let Some(Ok((mut t, mut f, mut c))) = kids.first().map(|&e| texts.get_mut(e)) {
            if t.0 != name {
                t.0 = name.to_string();
            }
            let size = FontSize::Px(size * px);
            if f.font_size != size {
                f.font_size = size;
            }
            let [r, g, b] = p.rgb.map(|c| (c / 128.0).min(1.0));
            c.set_if_neq(TextColor(Color::srgba(r, g, b, p.alpha / 128.0)));
        }
        vis.set_if_neq(Visibility::Inherited);
    }
}

/// Spawns the tags once the panel's colours are in.
fn setup(mut commands: Commands, g: Res<Game>, names: Option<Res<Names>>, colours: Option<Res<Colours>>, mut done: Local<bool>) {
    let (false, Some(names), Some(colours)) = (*done, names, colours) else { return };
    *done = true;
    for i in 0..g.players.len() {
        let Some(name) = names.get(i) else { continue };
        let [r, gg, b] = colours.0[i].map(|c| (c as f32 / 128.0).min(1.0));
        let colour = Color::srgb(r, gg, b);
        let text = commands
            .spawn((
                Text::new(name.to_string()),
                TextFont::from_font_size(16.0),
                TextColor(Color::WHITE),
                TextShadow { offset: Vec2::splat(1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.7) },
                TextLayout::no_wrap(),
            ))
            .id();
        let body = commands.spawn((Node::default(), BackgroundColor(colour))).add_child(text).id();
        // the pointer: a square turned 45° whose lower half shows under the body
        let pointer = commands.spawn((Node::default(), UiTransform::from_rotation(Rot2::degrees(45.0)), BackgroundColor(colour), ZIndex(-1))).id();
        let root = Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            width: Val::Percent(ROOM),
            ..default()
        };
        commands.spawn((Tag { i, body, text, pointer }, root, GlobalZIndex(1), Visibility::Hidden)).add_children(&[body, pointer]);
    }
}

/// A view point (fractions of the 3D view, y down) → the HUD's (`widescreen::in_view`'s turn for a sideways phone
/// match), as fractions of the HUD view.
fn to_hud(p: Vec2, turned: bool) -> Vec2 {
    if turned { Vec2::new(1.0 - p.y, p.x) } else { p }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn draw(
    g: Res<Game>,
    names: Option<Res<Names>>,
    time: Res<Time<Fixed>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform, &Transform), With<crate::Orbit>>,
    mut markers: Query<(&Marker, &mut Visibility), Without<Tag>>,
    mut tags: Query<(&Tag, &mut Node, &mut Visibility), Without<Marker>>,
    mut nodes: Query<&mut Node, Without<Tag>>,
    mut fonts: Query<&mut TextFont>,
) {
    let Some(names) = names else { return };
    let (Ok(win), Ok((camera, gt, tr))) = (window.single(), cam.single()) else { return };
    let on = markers::shown(&g);
    // a named player's marker gives way to the tag
    for (m, mut vis) in &mut markers {
        if names.get(m.0).is_some() {
            *vis = Visibility::Hidden;
        }
    }
    let turned = widescreen::turned(widescreen::view(win.size()).size());
    // the HUD's logical size (`hud_gamma` draws it at the view's size, turned when `turned`)
    let hud = widescreen::view_size(win);
    let a = time.overstep_fraction();
    let t = g.cam.view.fov.tan();
    for (tag, mut node, mut vis) in &mut tags {
        let Some(p) = g.players.get(tag.i).filter(|_| on) else {
            *vis = Visibility::Hidden;
            continue;
        };
        // the marker's anchor and height (`markers::draw`)
        let at = Vec3::from(p.prev).lerp(Vec3::from(p.pos), a);
        let anchor = Vec3::new(at.x, g.reaches[tag.i].base + 0.65, -at.z);
        let z = (anchor - tr.translation).dot(*tr.forward());
        let height = 2.0 * 0.84 * markers::half_width(z, t);
        let (Some(foot), Some(top)) = (camera.world_to_ndc(gt, anchor), camera.world_to_ndc(gt, anchor + *tr.up() * height)) else {
            *vis = Visibility::Hidden;
            continue;
        };
        if z <= 0.0 {
            *vis = Visibility::Hidden;
            continue;
        }
        let frac = |n: Vec3| Vec2::new((n.x + 1.0) / 2.0, (1.0 - n.y) / 2.0);
        let (foot, top) = (to_hud(frac(foot), turned), to_hud(frac(top), turned));
        // the marker's on-screen height in HUD px: the tag takes the same
        let px = ((foot - top) * hud).length().max(4.0);
        let font = px * 0.42;
        let point = px * 0.2;
        if let Ok(mut b) = nodes.get_mut(tag.body) {
            b.padding = UiRect::axes(Val::Px(font * 0.45), Val::Px(font * 0.1));
            b.border_radius = BorderRadius::all(Val::Px(font * 0.3));
        }
        if let Ok(mut n) = nodes.get_mut(tag.pointer) {
            (n.width, n.height) = (Val::Px(point * 1.4), Val::Px(point * 1.4));
            n.margin = UiRect::top(Val::Px(-point * 0.7));
        }
        if let Ok(mut f) = fonts.get_mut(tag.text)
            && !matches!(f.font_size, FontSize::Px(s) if (s - font).abs() <= 0.25)
        {
            f.font_size = FontSize::Px(font);
        }
        // pointer tip on the anchor: the root's bottom edge, its centre on the anchor's x
        let tip = point * 0.7 * std::f32::consts::SQRT_2 - point * 0.7;
        node.left = Val::Percent(foot.x * 100.0 - ROOM / 2.0);
        node.top = Val::Auto;
        node.bottom = Val::Percent((1.0 - foot.y + tip / hud.y) * 100.0);
        *vis = Visibility::Inherited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let n = Names::parse("ANNA,,BO B");
        assert_eq!(n.get(0), Some("ANNA"));
        assert_eq!(n.get(1), None);
        assert_eq!(n.get(2), Some("BO B"));
        assert_eq!(n.get(3), None);
        assert_eq!(to_hud(Vec2::new(0.25, 0.1), true), Vec2::new(0.9, 0.25));
    }

    /// A named human's slot label comes out of the quads; a nameless one's and a COM's stay.
    #[test]
    fn takes_named_labels() {
        let names = Names::parse("ANNA,,BO");
        // players 0..3 on slots 1, 0, 4 (COM), and slot labels for each plus another quad
        let mut quads = vec![(Some(1), 10.0), (Some(0), 20.0), (Some(4), 30.0), (None, 40.0)];
        let got = take(&mut quads, Some(&names), &[1, 0, 4], |q| q.0, |q| ([q.1, 0.0, 40.0, 24.0], [128.0; 3], 128.0));
        assert_eq!(got.iter().map(|p| (p.player, p.dst[0])).collect::<Vec<_>>(), [(0, 10.0)]);
        assert_eq!(quads.iter().map(|q| q.1).collect::<Vec<_>>(), [20.0, 30.0, 40.0]);
        assert!(take(&mut quads, None, &[1, 0, 4], |q| q.0, |q| ([q.1; 4], [0.0; 3], 0.0)).is_empty());
    }
}
