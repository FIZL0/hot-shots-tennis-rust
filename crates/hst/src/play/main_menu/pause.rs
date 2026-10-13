//! The main menu's Settings screens over a paused match (B126, remaster-only by request): the pause menu's "Settings"
//! opens them as the main menu draws them (same rows, art, input and `settings.txt`), Back on the Settings screen
//! returns to the pause menu. Changes go live where the match can take them: the graphics, widescreen, match music and
//! controls; upscaled textures and menu music take effect from the next match / the menu. The match stays paused
//! underneath (its fixed clock held by `menu.rs`), so the sim and its RNG never see any of it.
//!
//! Mouse and touch (here and on the pause dialog): a click or tap on a row chooses it, the right button is Back.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use hst_data::{iso::Iso, xb::Archive};
use std::sync::Arc;

use super::super::widescreen;
use super::{Art, Menu, POOL, Screen, Settings, Slot, Text, TickerClip, Ticker, slide};
use crate::Args;
use crate::audio::{MusicBanks, Sound, SoundBank};

/// The pause menu's Settings screens are up; `fresh` the frame they opened (its press is spent).
#[derive(Resource, Default)]
pub(in crate::play) struct PauseSettings {
    pub open: bool,
    fresh: bool,
}

impl PauseSettings {
    pub(in crate::play) fn open(&mut self) {
        *self = PauseSettings { open: true, fresh: true };
    }
}

#[derive(Component)]
pub(in crate::play) struct Root;

/// This overlay's systems, after the pause menu's step (`menu.rs`).
#[derive(SystemSet, Clone, PartialEq, Eq, Hash, Debug)]
pub(in crate::play) struct Systems;

/// What the overlay draws on: the Settings screens' quads only need the font, the hand, `title_option`,
/// `wall_option`, `option_00` and our pill (9 quads a pill since its rim, ring and fill are layered).
const POOL_HERE: usize = 480;

/// One sheet out of a disc archive.
fn sheet(iso: &mut Iso, xb: &str, name: &str) -> Vec<u8> {
    let data = iso.read(xb).expect("menu archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name)).unwrap_or_else(|| panic!("{name} in {xb}"));
    arc.read(e).expect("archive bytes")
}

/// `s` in `word.tm2`, white, centred on a 256×32 sheet as one of `i_dialog_01`'s rows (the dialog has no "Settings"),
/// set again in the remastered font (`menu_art::relabel`) unless `--original`.
pub(in crate::play) fn label(iso: &mut Iso, images: &mut Assets<Image>, s: &str) -> Handle<Image> {
    let font = sheet(iso, "MENU/MENU04.XB0", "/word.tm2");
    let glyphs = super::glyphs(&font);
    let pic = hst_data::tim2::decode_alpha8(&font).expect("TIM2").remove(0);
    let (w, h) = (256usize, 32usize);
    // the glyphs' coverage, then white over a 2-px dark outline as the dialog's own rows are drawn
    let width: f32 = s.chars().map(|c| glyphs.get(&c).map_or(10.0, |g| g.2 + 2.0)).sum::<f32>() - 2.0;
    // B171: words wider than the row are set on a wider one (room for the outline), then shrunk to fit it
    let wide = w.max(width.ceil() as usize + 4);
    let mut ink = vec![0f32; wide * h];
    let mut x = ((wide as f32 - width) / 2.0).max(0.0) as usize;
    for c in s.chars() {
        let Some(&(u, v, gw)) = glyphs.get(&c) else {
            x += 10;
            continue;
        };
        for dy in 0..h {
            for dx in 0..gw as usize {
                if x + dx < wide {
                    ink[dy * wide + x + dx] = pic.rgba[((v as usize + dy) * pic.width as usize + u as usize + dx) * 4 + 3] as f32 / 255.0;
                }
            }
        }
        x += gw as usize + 2;
    }
    let ink = shrink(&ink, wide, w, h);
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let near = (-2..=2i32).flat_map(|dy| (-2..=2i32).map(move |dx| (dx, dy))).filter(|(dx, dy)| dx * dx + dy * dy <= 5);
            let edge = near.filter_map(|(dx, dy)| (x + dx >= 0 && y + dy >= 0 && x + dx < w as i32 && y + dy < h as i32).then(|| ink[((y + dy) * w as i32 + x + dx) as usize])).fold(0f32, f32::max);
            let a = ink[(y * w as i32 + x) as usize];
            let c = (30.0 + 225.0 * a) as u8;
            rgba[(y as usize * w + x as usize) * 4..][..4].copy_from_slice(&[c, c, c, (edge.max(a) * 255.0) as u8]);
        }
    }
    if let Some(h) = super::super::menu_art::relabel(images, w, h, &rgba, s) {
        return h;
    }
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    images.add(Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, rgba, TextureFormat::Rgba8UnormSrgb, bevy::asset::RenderAssetUsages::RENDER_WORLD))
}

/// A `wide`×`h` coverage row scaled by `w / wide` both ways (box filter), centred on a `w`×`h` one.
fn shrink(ink: &[f32], wide: usize, w: usize, h: usize) -> Vec<f32> {
    if wide == w {
        return ink.to_vec();
    }
    let f = w as f32 / wide as f32;
    let y0 = h as f32 * (1.0 - f) / 2.0;
    let mut out = vec![0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            // 4×4 samples over the source square under this pixel
            let mut sum = 0.0;
            for k in 0..16 {
                let (sx, sy) = ((x as f32 + (k % 4) as f32 / 4.0 + 0.125) / f, (y as f32 + (k / 4) as f32 / 4.0 + 0.125 - y0) / f);
                if sy >= 0.0 && (sy as usize) < h {
                    sum += ink[sy as usize * wide + (sx as usize).min(wide - 1)];
                }
            }
            out[y * w + x] = sum / 16.0;
        }
    }
    out
}

fn setup(mut commands: Commands, args: Res<Args>, mut images: ResMut<Assets<Image>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let solid = images.add(Image::new_fill(
        bevy::render::render_resource::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        &[255; 4],
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    ));
    let font = sheet(&mut iso, "MENU/MENU04.XB0", "/word.tm2");
    let hand = sheet(&mut iso, "AZUMA/INPANE/INPANE.XB0", "/hsm_yubi.tm2");
    let mut art = vec![solid, super::super::menu_art::image(&mut images, &font, "word"), super::panel::image(&mut images, &hand)];
    // `Tex::Sheet(k)` is `art[3 + k]`: only the Settings screens' sheets, the rest left empty
    for (xb, name) in &super::SHEETS[..10] {
        art.push(if name.ends_with("option") || *name == "option_00" { super::super::menu_art::image(&mut images, &sheet(&mut iso, xb, &format!("/{name}.tm2")), name) } else { Handle::default() });
    }
    // `Tex::Pill` is the last sheet, as in the main menu
    art.push(super::super::menu_art::pill_layers(&mut images));
    let msg = super::messages(&sheet(&mut iso, "MENU/MENU00B.XB0", "/message0.dat"));
    let text = Text { msg, glyphs: super::glyphs(&font), grades: [[0; 5]; 14], style: [0; 14], hands: [1.0; 14], ranks: Vec::new(), mods: Vec::new() };
    let bank = SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd").map(Arc::new);
    commands.insert_resource(Art(art, bank, text));
    commands.insert_resource(Menu { screen: Screen::Settings, ..default() });
    let full = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    // over the pause screen (`GlobalZIndex(10)`)
    commands.spawn((full, GlobalZIndex(11), Visibility::Hidden, Root)).with_children(|p| {
        for i in 0..POOL_HERE.min(POOL) {
            p.spawn((Slot(i), ImageNode { image_mode: NodeImageMode::Stretch, ..default() }, Node { position_type: PositionType::Absolute, ..default() }, Visibility::Hidden));
        }
        let clip = Node { position_type: PositionType::Absolute, overflow: Overflow::clip(), ..default() };
        p.spawn((TickerClip, clip, ZIndex(POOL_HERE as i32), Visibility::Hidden)).with_child((
            bevy::ui::widget::Text::new(""),
            TextColor(Color::srgb_u8(223, 223, 223)),
            TextLayout::no_wrap(),
            Node { position_type: PositionType::Absolute, ..default() },
        ));
    });
}

/// Opening: the Settings screen, first row, with what `settings.txt` holds now.
fn opened(ps: Res<PauseSettings>, args: Res<Args>, mut menu: ResMut<Menu>, mut slide: ResMut<slide::Slide>, mut root: Query<&mut Visibility, With<Root>>) {
    if ps.fresh {
        let settings = crate::store::read_to_string(Settings::path(&args.iso)).map_or(menu.settings, |t| Settings::parse(&t));
        let screen = match std::env::var("HST_PAUSE_SETTINGS").as_deref() {
            Ok("graphics") => Screen::Graphics,
            Ok("sound") => Screen::Sound,
            Ok("controls") => Screen::Controls,
            _ => Screen::Settings,
        };
        *menu = Menu { screen, settings, ..default() };
        *slide = slide::Slide::default();
    }
    for mut v in &mut root {
        v.set_if_neq(if ps.open { Visibility::Inherited } else { Visibility::Hidden });
    }
}

/// The 640×448 point under a window position, on the centred 4:3 screen the rows are drawn on.
fn screen_point(win: &Window, p: Vec2) -> Vec2 {
    #[allow(unused_mut)]
    let mut size = Vec2::new(win.width().max(1.0), win.height().max(1.0));
    // the browser measures in CSS px (`pointer.rs`)
    #[cfg(target_arch = "wasm32")]
    if let Some(css) = crate::web::page_size() {
        size = css;
    }
    let share = widescreen::share(size.x / size.y);
    let x = (p.x / size.x * 100.0 - 50.0 * (1.0 - share)) * 6.4 / share;
    Vec2::new(x, p.y / size.y * 448.0)
}

/// This frame's left click or first tap, and whether the right button was clicked.
pub(in crate::play) fn pointer_press(window: &Query<&Window, With<PrimaryWindow>>, mouse: &ButtonInput<MouseButton>, touches: &Touches) -> (Option<Vec2>, bool) {
    let Ok(win) = window.single() else { return (None, false) };
    let at = touches.iter_just_pressed().next().map(|t| t.position()).or_else(|| mouse.just_pressed(MouseButton::Left).then(|| win.cursor_position()).flatten());
    (at.map(|p| screen_point(win, p)), mouse.just_pressed(MouseButton::Right))
}

/// The 640×448 point under the mouse, if it's over the window.
pub(in crate::play) fn cursor_point(window: &Query<&Window, With<PrimaryWindow>>) -> Option<Vec2> {
    let win = window.single().ok()?;
    win.cursor_position().map(|p| screen_point(win, p))
}

/// Back on the Settings screen leaves for the main menu's first screen: here, the pause menu. And the settings the
/// match can take live: widescreen and the court's music (the graphics and controls `super::step` sets already).
fn after(
    mut ps: ResMut<PauseSettings>,
    menu: Res<Menu>,
    mut args: ResMut<Args>,
    game: Option<Res<super::super::Game>>,
    sound: Option<Res<Sound>>,
    music: Option<ResMut<MusicBanks>>,
) {
    if !ps.open {
        return;
    }
    ps.fresh = false;
    let [wide, _, _, match_music, _] = menu.settings.0;
    widescreen::WIDE.store(wide, std::sync::atomic::Ordering::Relaxed);
    if match_music != args.music
        && let (Some(game), Some(sound), Some(mut music)) = (game, sound, music)
    {
        args.music = match_music;
        if match_music {
            // `play.rs setup`'s pick: the court's track, else the default one
            let stage = game.stage as usize;
            let name = if (1..=11).contains(&stage) { format!("bgmg_{stage:02}") } else { "bgmg_14".into() };
            music.bgm = Iso::open(&args.iso).ok().and_then(|mut iso| SoundBank::music(&mut iso, "Court", &name)).map(|(b, m)| (Arc::new(b), m));
            if let Some((bank, mid)) = &music.bgm {
                sound.music(bank, mid, 55);
            }
        } else {
            music.bgm = None;
            sound.stop_music();
        }
    }
    if menu.screen == Screen::Main {
        ps.open = false;
    }
}

pub(in crate::play) fn plugin(app: &mut App) {
    #[cfg(target_arch = "wasm32")]
    app.add_systems(Update, touch_menu);
    let open = |ps: Res<PauseSettings>| ps.open;
    let live = |ps: Res<PauseSettings>| ps.open && !ps.fresh;
    app.init_resource::<PauseSettings>()
        .init_resource::<super::pointer::Pending>()
        .init_resource::<super::pointer::Hits>()
        .init_resource::<Ticker>()
        .init_resource::<slide::Slide>()
        // `super::draw` reads the custom page's mod icons (none in a match); without it the draw panicked (B139)
        .init_resource::<super::Icons>()
        .init_resource::<super::mods_screen::Shelf>()
        .add_systems(PostStartup, setup)
        .add_systems(
            Update,
            (opened, super::pointer::pointer.run_if(live), super::step.run_if(live), after, (super::draw, super::ticker_text).run_if(open))
                .chain()
                .in_set(Systems),
        );
}

/// B221/B244: the phone's on-screen stick keeps to its corner while the pause or result screen is up, so taps reach the
/// menu under it.
#[cfg(target_arch = "wasm32")]
fn touch_menu(w: &World, mut was: Local<Option<bool>>) {
    let on = super::super::menu::paused(w) || w.get_resource::<super::super::Game>().is_some_and(|g| g.match_stats.result.is_some());
    if *was != Some(on) {
        *was = Some(on);
        crate::web::touch_menu(on);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{Press, controls};

    /// A full 512×32 row shrinks to half height, centred, edge to edge (B171).
    #[test]
    fn shrink_fits() {
        let out = shrink(&vec![1.0; 512 * 32], 512, 256, 32);
        let col: Vec<f32> = (0..32).map(|y| out[y * 256 + 128]).collect();
        assert!(col[..8].iter().chain(&col[24..]).all(|&a| a == 0.0) && col[8..24].iter().all(|&a| a == 1.0), "{col:?}");
        assert!(out[16 * 256] == 1.0 && out[16 * 256 + 255] == 1.0);
    }

    /// The window → 640×448 mapping undoes the centred 4:3 placement at any aspect.
    #[test]
    fn window_to_screen() {
        let mut win = Window::default();
        win.resolution.set(1920.0, 1080.0);
        // 16:9: the 4:3 screen is the middle 1440 px
        assert_eq!(screen_point(&win, Vec2::new(240.0, 540.0)), Vec2::new(0.0, 224.0));
        assert_eq!(screen_point(&win, Vec2::new(1680.0, 1080.0)), Vec2::new(640.0, 448.0));
        win.resolution.set(800.0, 600.0);
        assert_eq!(screen_point(&win, Vec2::new(400.0, 300.0)), Vec2::new(320.0, 224.0));
    }

    /// A row changes and saves; Back on the Settings screen goes to `Screen::Main`, which `after` takes as closing.
    #[test]
    fn back_returns_to_pause() {
        let mut m = Menu { screen: Screen::Settings, ..default() };
        let fx = m.step(super::super::Dev::Keys, Press { ok: true, ..default() });
        assert!(!m.settings.0[0] && fx.contains(&super::super::Effect::SaveSettings));
        m.step(super::super::Dev::Keys, Press { back: true, ..default() });
        assert_eq!(m.screen, Screen::Main);
    }

    /// B139: the overlay's systems run in an app with only what a match has (no main menu): open Settings, toggle
    /// Menu Music (not Widescreen: `after` stores that in a global the parallel widescreen tests read), Back to the
    /// pause menu, no panic. `settings.txt` goes beside the ISO, so a linked copy in a temp directory keeps the
    /// user's. Needs the ISO at the repository root, skipped without.
    #[test]
    fn open_change_back_in_a_match() {
        let iso = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");
        if !std::path::Path::new(iso).is_file() {
            return eprintln!("no ISO, skipped");
        }
        let dir = std::env::temp_dir().join(format!("hst-b139-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let link = dir.join("game.iso");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(std::fs::canonicalize(iso).unwrap(), &link).unwrap();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), bevy::input::InputPlugin));
        app.init_asset::<Image>().init_resource::<bevy::clipboard::Clipboard>().init_resource::<crate::graphics::Graphics>();
        app.insert_resource(Args { iso: link.to_string_lossy().into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.add_plugins(controls::plugin);
        plugin(&mut app);
        app.update();
        app.world_mut().resource_mut::<PauseSettings>().open();
        app.update();
        let was = app.world().resource::<Menu>().settings.0[2];
        app.world_mut().resource_mut::<super::super::pointer::Pending>().0.push((super::super::pointer::Act::Row(2, super::super::pointer::OK), true));
        app.update();
        assert_eq!(app.world().resource::<Menu>().settings.0[2], !was, "Menu Music didn't change");
        assert!(std::fs::read_to_string(dir.join("settings.txt")).is_ok(), "settings not saved");
        app.world_mut().resource_mut::<super::super::pointer::Pending>().0.push((super::super::pointer::press(|k| k.back = true), true));
        app.update();
        assert!(!app.world().resource::<PauseSettings>().open, "Back didn't return to the pause menu");
        app.update();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
