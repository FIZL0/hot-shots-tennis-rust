//! B371: player names over the players in a match. A named player's "1P".."4P" marker (`markers`) is swapped for
//! the name set as the original's own tag (B541, [`Style::Head`]): white words leaning right in a grey outline with
//! the marker's pointer under them, tinted with the player's panel colour, a camera-facing quad hung at the marker's
//! spot (the pointer's tip on its anchor), as tall as the marker and as wide as the name needs. Unnamed players keep
//! the original's marker. The names come from the main menu's name picker (`--names a,,b`, by match slot); without
//! any the match looks as before. The point boards' plates, the final board and the result screen set the name where
//! the slot label was (`take`, `labels`), as that label is drawn ([`Style::Plate`]). Drawing only: nothing here
//! touches the sim.

use bevy::prelude::*;
use tiny_skia::{Color as SkColor, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform as SkTransform};

use super::markers::{self, Marker};
use super::menu_art::{self, Look};
use super::panel::Colours;
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

/// B541: which of the original's seat tags a name is set as. The disc has these tags' words only ("1P".."4P",
/// "COM"), so a name is set in the menus' remastered font (`menu_art`) in the tag's colours, outline and size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    /// Over the player's head (`i_playerinfo`'s 40×32 cells): white leaning words (3 px across their height) in a
    /// grey (64) 2-px outline, 18 px tall with it, from 1 px down; the pointer under them.
    Head,
    /// The point boards' slot labels (`inpane_p`) and the result screen's (the stats sheet): upright white words in
    /// a near-black (30) 2-px outline, 19 px tall with it.
    Plate,
}

impl Style {
    fn look(self) -> Look {
        let (edge, slant) = match self {
            Style::Head => (64, 0.27),
            Style::Plate => (30, 0.0),
        };
        Look { top: [255; 3], bottom: [255; 3], edge: [edge; 3], ring: [0; 3], ew: 2.0, rw: 0.0, slant }
    }

    /// The words' box (outline included) and its top in the tag, disc px.
    fn words(self) -> (f32, f32) {
        match self {
            Style::Head => (18.0, 1.0),
            Style::Plate => (PLATE_H, 0.0),
        }
    }
}

/// The plate tag's words' box height (disc px, outline included): its image's height less an accent's or a
/// descender's rows.
pub const PLATE_H: f32 = 19.0;
/// How narrow a plate's words get (`squeeze`) before a long name shrinks instead: a condensed face, still the tag's
/// height.
const CONDENSE: f32 = 0.7;
/// A tag's room for the words' antialiased edge either side (disc px).
const PAD: f32 = 1.0;
/// The head tag's cell (`i_playerinfo`): no narrower than the original's, as tall.
const HEAD: [f32; 2] = [40.0, 32.0];
/// A pill's name over its label and the (human's empty) rank row: this much the label's height.
const BIG: f32 = 1.4;

/// `name` set as `style`'s tag: its size in disc px (the head tag's pointer tip mid-bottom) and how far above the
/// tag's own top an accent reaches (the image's extra rows, included in the size).
pub fn tag_size(name: &str, style: Style) -> ([f32; 2], f32) {
    let font = ab_glyph::FontRef::try_from_slice(menu_art::HEAVY).expect("menu font");
    let l = style.look();
    let (h, _) = style.words();
    let m = l.ew + l.rw;
    let (Some(path), Some(caps)) = (menu_art::text_path(&font, name), menu_art::text_path(&font, "H")) else {
        return (if style == Style::Head { HEAD } else { [2.0 * (m + PAD), h] }, 0.0);
    };
    let (b, cap) = (path.bounds(), -caps.bounds().top());
    let sy = (h - 2.0 * m) / cap;
    let w = b.width() * sy + l.slant * (h - 2.0 * m) + 2.0 * (m + PAD);
    // accents over the caps, descenders under the baseline (font y is 0 at the baseline, down positive); a round
    // glyph's or a digit's overshoot (under a px) stays in the outline's rows
    let above = ((-b.top() - cap) * sy - 1.0).max(0.0).ceil();
    let below = (b.bottom() * sy - 1.0).max(0.0).ceil();
    match style {
        // a descender runs into the pointer, as it would on the disc's art
        Style::Head => ([w.max(HEAD[0]), HEAD[1] + above], above),
        Style::Plate => ([w, h + above + below], above),
    }
}

/// `name` set as `style`'s tag at `menu_art::K`× ([`tag_size`]; its words `squeeze` as wide, ≤ 1), white and grey for
/// a tint to colour as the original's art.
pub fn tag_image(name: &str, style: Style, squeeze: f32) -> Image {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let font = ab_glyph::FontRef::try_from_slice(menu_art::HEAVY).expect("menu font");
    let ([w, h], above) = tag_size(name, style);
    let w = w * squeeze.min(1.0);
    let k = menu_art::K as f32;
    let mut out = Pixmap::new((w * k).ceil() as u32, (h * k).ceil() as u32).expect("tag pixmap");
    let (wh, top) = style.words();
    if style == Style::Head {
        pointer(&mut out, w / 2.0, above);
    }
    menu_art::stamp(&mut out, &font, [PAD, top + above, w - 2.0 * PAD, wh], name, &style.look());
    let px = out.pixels().iter().flat_map(|c| {
        let c = c.demultiply();
        [c.red(), c.green(), c.blue(), c.alpha()]
    });
    Image::new(
        Extent3d { width: out.width(), height: out.height(), depth_or_array_layers: 1 },
        TextureDimension::D2,
        px.collect(),
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
}

/// The head tag's pointer under the words, its middle at `cx` (disc px), `above` rows down: as `i_playerinfo`'s, a
/// white triangle 14 px across at row 22 narrowing to its tip at row 29, in the words' 2-px grey outline.
fn pointer(out: &mut Pixmap, cx: f32, above: f32) {
    let k = menu_art::K as f32;
    let mut pb = PathBuilder::new();
    pb.move_to((cx - 7.0) * k, (22.0 + above) * k);
    pb.line_to((cx + 7.0) * k, (22.0 + above) * k);
    pb.line_to(cx * k, (29.0 + above) * k);
    pb.close();
    let path = pb.finish().expect("pointer");
    let mut paint = Paint { anti_alias: true, ..Paint::default() };
    paint.set_color(SkColor::from_rgba8(64, 64, 64, 255));
    out.stroke_path(&path, &paint, &Stroke { width: 4.0 * k, line_join: LineJoin::Miter, line_cap: LineCap::Butt, ..Stroke::default() }, SkTransform::identity(), None);
    paint.set_color(SkColor::WHITE);
    out.fill_path(&path, &paint, FillRule::Winding, SkTransform::identity(), None);
}

/// One named player's head tag: its quad's material, the tag's size (disc px).
#[derive(Component)]
struct Tag {
    i: usize,
    material: Handle<StandardMaterial>,
    size: [f32; 2],
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Plates>()
        .add_systems(Update, (setup, draw).chain().after(markers::draw))
        // after every Update draw that fills `Plates`
        .add_systems(PostUpdate, labels.before(bevy::ui::UiSystems::Prepare));
}

/// A named player's slot label on a HUD this frame (640×448 rect, its cell on its sheet, tint and alpha out of 128):
/// `labels` sets the name there instead.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Plate {
    player: usize,
    dst: [f32; 4],
    src: [f32; 4],
    rgb: [f32; 3],
    alpha: f32,
    /// Disc px right of the label's cell that are clear for a left-half name (a human's row has no rank under it).
    extra: f32,
}

/// Which HUD a pool of labels is on.
pub(super) const BOARDS: usize = 0;
pub(super) const RESULT: usize = 1;
/// The score panel before a serve (`panel.rs`): pinned to the screen's sides, as its quads.
pub(super) const PANEL: usize = 2;

/// Each HUD's plates this frame (`BOARDS`, `RESULT`, `PANEL`).
#[derive(Resource, Default)]
pub(super) struct Plates(pub(super) [Vec<Plate>; 3]);

/// A name label in a HUD's pool: the HUD and its index.
#[derive(Component)]
struct Label(usize, usize);

/// Spawns `hud`'s labels under its 4:3 root, after its quads so they draw over them.
pub(super) fn pool(p: &mut bevy::ecs::hierarchy::ChildSpawnerCommands, hud: usize) {
    for k in 0..4 {
        p.spawn((Label(hud, k), ImageNode { image_mode: NodeImageMode::Stretch, ..default() }, Node { position_type: PositionType::Absolute, ..default() }, Visibility::Hidden));
    }
}

/// Takes the named players' slot labels out of `quads`: `label` gives a quad's slot if it is a slot label, `parts`
/// its rect, cell, tint and alpha; `slots` is each match player's slot (humans 0..3, unique). The rest stay as they
/// are.
pub(super) fn take<Q>(
    quads: &mut Vec<Q>,
    names: Option<&Names>,
    slots: &[usize],
    extra: f32,
    label: impl Fn(&Q) -> Option<usize>,
    parts: impl Fn(&Q) -> ([f32; 4], [f32; 4], [f32; 3], f32),
) -> Vec<Plate> {
    let Some(names) = names else { return Vec::new() };
    let mut out = Vec::new();
    quads.retain(|q| {
        let player = label(q).filter(|&s| s < 4).and_then(|s| slots.iter().position(|&x| x == s)).filter(|&i| names.get(i).is_some());
        let Some(player) = player else { return true };
        let (dst, src, rgb, alpha) = parts(q);
        out.push(Plate { player, dst, src, rgb, alpha, extra });
        false
    });
    out
}

/// A slot label's words in its cell (disc px: left, top, right of their ink, outline included): `inpane_p`'s 40-px
/// cells or the stats sheet's 48-px rows.
fn ink(src: [f32; 4]) -> [f32; 3] {
    if src[2] == 48.0 { [0.0, 2.5, 32.0] } else { [5.0, 4.0, 33.0] }
}

/// Where a plate's name goes on its HUD (640×448), and how much its words are squeezed: the tag of `size` (disc px,
/// `above` of it over the words' top) scaled as the label's cell is, its words where the label's were, from their
/// left edge on the screen's left half, to their right edge on the right half (where the face is on the other side).
/// A name wider than the cell has room for from there (the next column, the set count) is condensed to fit, down to
/// [`CONDENSE`], then shrinks at that aspect round the words' middle.
fn place(p: &Plate, size: [f32; 2], above: f32) -> ([f32; 4], f32) {
    let ([x, y, w, h], [_, _, sw, sh]) = (p.dst, p.src);
    let (kx, ky) = (w / sw, h / sh);
    let [l, t, r] = ink(p.src);
    // a pill's label (`inpane_p`): a named player is human, so no rank row under it; the name takes both rows
    let (big, t) = if sw == 40.0 { (BIG, sh - PLATE_H / 2.0) } else { (1.0, t) };
    let (size, above) = ([size[0] * big, size[1] * big], above * big);
    let right = x + w / 2.0 > 320.0;
    let room = if right { r + PAD } else { sw - l + PAD + p.extra };
    // in twentieths (down), so a HUD's few cells make few images
    let squeeze = ((room / size[0] * 20.0 + 1e-3).floor() / 20.0).clamp(CONDENSE, 1.0);
    let f = (room / (size[0] * squeeze)).min(1.0);
    let tw = size[0] * squeeze * f * kx;
    let left = if right { x + (r + PAD) * kx - tw } else { x + (l - PAD) * kx };
    let mid = y + (t + PLATE_H / 2.0) * ky;
    ([left, mid - (above + big * PLATE_H / 2.0) * f * ky, tw, size[1] * f * ky], squeeze)
}

/// Sets each HUD's plates' names where their labels were ([`place`]), in the label's tint.
#[allow(clippy::type_complexity)]
fn labels(
    plates: Res<Plates>,
    names: Option<Res<Names>>,
    mut images: ResMut<Assets<Image>>,
    mut sizes: Local<std::collections::HashMap<String, ([f32; 2], f32)>>,
    mut made: Local<std::collections::HashMap<(String, u8), Handle<Image>>>,
    mut q: Query<(&Label, &mut Node, &mut ImageNode, &mut Visibility)>,
    window: Query<&Window>,
) {
    let aspect = window.single().map_or(4.0 / 3.0, super::widescreen::aspect);
    for (Label(h, k), mut node, mut img, mut vis) in &mut q {
        let Some((p, name)) = plates.0[*h].get(*k).and_then(|p| Some((p, names.as_ref()?.get(p.player)?))) else {
            vis.set_if_neq(Visibility::Hidden);
            continue;
        };
        let (size, above) = *sizes.entry(name.to_string()).or_insert_with(|| tag_size(name, Style::Plate));
        let ([x, y, w, hh], squeeze) = place(p, size, above);
        let handle = made.entry((name.to_string(), (squeeze * 20.0).round() as u8)).or_insert_with(|| images.add(tag_image(name, Style::Plate, squeeze))).clone();
        if *h == PANEL {
            let (left, width) = super::widescreen::pin(x, w, super::widescreen::share(aspect));
            let (top, height) = super::widescreen::vplace(y, hh, aspect, super::widescreen::Anchor::Sides);
            super::widescreen::put(&mut node, left, top, width, height);
        } else {
            let placed = Node { left: Val::Percent(x / 6.4), top: Val::Percent(y / 4.48), width: Val::Percent(w / 6.4), height: Val::Percent(hh / 4.48), ..node.clone() };
            node.set_if_neq(placed);
        }
        if img.image != handle {
            img.image = handle;
        }
        let [r, g, b] = p.rgb.map(|c| (c / 128.0).min(1.0));
        let colour = Color::srgba(r, g, b, p.alpha / 128.0);
        if img.color != colour {
            img.color = colour;
        }
        vis.set_if_neq(Visibility::Inherited);
    }
}

/// Spawns the named players' head tags: each its own quad (bottom edge on the anchor) and material, as the markers.
fn setup(
    mut commands: Commands,
    g: Res<Game>,
    names: Option<Res<Names>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut done: Local<bool>,
) {
    let (false, Some(names)) = (*done, names) else { return };
    *done = true;
    let quad = meshes.add(Rectangle::new(1.0, 1.0).mesh().build().translated_by(Vec3::Y * 0.5));
    for i in 0..g.players.len() {
        let Some(name) = names.get(i) else { continue };
        let (size, _) = tag_size(name, Style::Head);
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(images.add(tag_image(name, Style::Head, 1.0))),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            depth_bias: markers::LAST,
            ..default()
        });
        commands.spawn((Tag { i, material: material.clone(), size }, Mesh3d(quad.clone()), MeshMaterial3d(material), Transform::default(), Visibility::Hidden));
    }
}

/// Hangs each head tag where its player's marker would be (`markers::draw`), the marker's height and the tag's
/// width at the marker's scale, tinted as the marker; the named players' markers stay hidden.
#[allow(clippy::type_complexity)]
fn draw(
    g: Res<Game>,
    names: Option<Res<Names>>,
    colours: Option<Res<Colours>>,
    time: Res<Time<Fixed>>,
    cam: Query<&Transform, (With<crate::Orbit>, Without<Marker>, Without<Tag>)>,
    mut markers: Query<(&Marker, &mut Visibility), Without<Tag>>,
    mut tags: Query<(&Tag, &mut Transform, &mut Visibility), Without<Marker>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(names) = names else { return };
    for (m, mut vis) in &mut markers {
        if names.get(m.0).is_some() {
            *vis = Visibility::Hidden;
        }
    }
    let (Ok(cam), Some(colours)) = (cam.single(), colours) else { return };
    let on = markers::shown(&g);
    let a = time.overstep_fraction();
    let t = g.cam.view.fov.tan();
    for (tag, mut tr, mut vis) in &mut tags {
        let Some(p) = g.players.get(tag.i).filter(|_| on) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let at = Vec3::from(p.prev).lerp(Vec3::from(p.pos), a);
        let anchor = Vec3::new(at.x, g.reaches[tag.i].base + 0.65, -at.z);
        let z = (anchor - cam.translation).dot(*cam.forward());
        let s = markers::half_width(z, t);
        tr.translation = anchor;
        tr.rotation = cam.rotation;
        // the marker's 40×32 cell is 2s × 1.68s
        tr.scale = Vec3::new(2.0 * s * tag.size[0] / HEAD[0], 2.0 * 0.84 * s * tag.size[1] / HEAD[1], 1.0);
        let [r, gg, b] = colours.0[tag.i].map(|c| c as f32 / 128.0);
        let colour = Color::srgb(r, gg, b);
        // written only when it changes (B328)
        if materials.get(&tag.material).is_some_and(|mat| mat.base_color != colour)
            && let Some(mut mat) = materials.get_mut(&tag.material)
        {
            mat.base_color = colour;
        }
        *vis = Visibility::Visible;
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
    }

    /// A named human's slot label comes out of the quads; a nameless one's and a COM's stay.
    #[test]
    fn takes_named_labels() {
        let names = Names::parse("ANNA,,BO");
        // players 0..3 on slots 1, 0, 4 (COM), and slot labels for each plus another quad
        let mut quads = vec![(Some(1), 10.0), (Some(0), 20.0), (Some(4), 30.0), (None, 40.0)];
        let got = take(&mut quads, Some(&names), &[1, 0, 4], 0.0, |q| q.0, |q| ([q.1, 0.0, 40.0, 24.0], [0.0, 0.0, 40.0, 24.0], [128.0; 3], 128.0));
        assert_eq!(got.iter().map(|p| (p.player, p.dst[0])).collect::<Vec<_>>(), [(0, 10.0)]);
        assert_eq!(quads.iter().map(|q| q.1).collect::<Vec<_>>(), [20.0, 30.0, 40.0]);
        assert!(take(&mut quads, None, &[1, 0, 4], 0.0, |q| q.0, |q| ([q.1; 4], [0.0; 4], [0.0; 3], 0.0)).is_empty());
    }

    /// B541: "1P" set as either tag comes out the size of the disc's own (the head cell 40×32; the plate's words
    /// 25–28 px across with their outline, 19 tall), a six-letter name is wider but no taller, and an accent or a
    /// descender gets its own rows. `HST_TAG_ART=DIR` saves the tags as PNGs to look at.
    /// A plate's name starts where the label's words did (left half) or ends where they did (right half), 1.4× the
    /// label's words over the label and the (empty) rank row; a long one shrinks into the cell's room round the
    /// middle.
    #[test]
    fn plates_fit_their_cells() {
        let plate = |x: f32, extra: f32| Plate { player: 0, dst: [x, 100.0, 80.0, 48.0], src: [0.0, 0.0, 40.0, 24.0], rgb: [128.0; 3], alpha: 128.0, extra };
        let close = |a: [f32; 4], b: [f32; 4]| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-3);
        // 30 wide at 1.4 = 42 in 36 + 40 of room: whole, its middle on the label's bottom edge (the pill's middle)
        let (r, squeeze) = place(&plate(100.0, 40.0), [30.0, PLATE_H], 0.0);
        assert!(close(r, [108.0, 148.0 - 26.6, 84.0, 53.2]) && squeeze == 1.0, "{r:?}");
        // right half: no extra, condensed to 0.8 of 42 into 34, ending where the words did
        let ([l, _, w, _], squeeze) = place(&plate(400.0, 40.0), [30.0, PLATE_H], 0.0);
        assert!((l + w - (400.0 + 34.0 * 2.0)).abs() < 1e-3 && squeeze == 0.8);
        // 72 wide (100.8) in 36: condensed to 0.7, then shrunk, its middle still the pill's
        let ([_, t, _, h], squeeze) = place(&plate(100.0, 0.0), [72.0, PLATE_H], 0.0);
        assert!(squeeze == 0.7 && (h - 2.0 * 26.6 * 36.0 / 70.56).abs() < 1e-3 && (t + h / 2.0 - 148.0).abs() < 1e-3, "{t} {h}");
    }

    #[test]
    fn tags_are_the_originals_size() {
        let (head, _) = tag_size("1P", Style::Head);
        assert_eq!(head, HEAD);
        let ([w, h], above) = tag_size("1P", Style::Plate);
        assert!((25.0..=32.0).contains(&w) && h == 19.0 && above == 0.0, "{w} {h} {above}");
        let ([w, h], _) = tag_size("WWWWWW", Style::Head);
        assert!(w > 80.0 && h == HEAD[1], "{w} {h}");
        let ([_, h], above) = tag_size("Éy", Style::Plate);
        assert!(above > 0.0 && h > 19.0 + above, "{h} {above}");
        for (name, style) in [("1P", Style::Head), ("ANNA", Style::Head), ("WWWWWW", Style::Head), ("Jégy", Style::Head), ("1P", Style::Plate), ("KIM-LI", Style::Plate), ("Jégy", Style::Plate)] {
            let img = tag_image(name, style, 1.0);
            let k = menu_art::K as f32;
            let ([w, h], _) = tag_size(name, style);
            assert_eq!((img.width(), img.height()), ((w * k).ceil() as u32, (h * k).ceil() as u32));
            let px = img.data.as_ref().unwrap();
            // white words, a darker outline: both there
            assert!(px.chunks(4).any(|c| c == [255, 255, 255, 255]) && px.chunks(4).any(|c| c[3] == 255 && c[0] < 100), "{name}");
            if let Ok(dir) = std::env::var("HST_TAG_ART") {
                let _ = image::save_buffer(format!("{dir}/{name}-{style:?}.png"), px, img.width(), img.height(), image::ColorType::Rgba8);
            }
        }
    }
}
