//! The main menu's Settings screens over a paused match (B126, remaster-only by request): the pause menu's "Settings"
//! opens them as the main menu draws them (same rows, art, input and `settings.txt`), Back on the Settings screen
//! returns to the pause menu. Changes go live where the match can take them: the graphics, widescreen, match music and
//! controls; texture replacements and menu music take effect from the next match / the menu. The match stays paused
//! underneath (its fixed clock held by `menu.rs`), so the sim and its RNG never see any of it.
//!
//! Mouse and touch (here and on the pause dialog): a click or tap on a row chooses it, the right button is Back.
//!
//! B254: the pause menu's "Controllers" (local matches) opens the main menu's controller assignment the same way, with
//! the match's seats: each human seat's device, the computer's seats fixed (`Menu::paused`). 1P's ✕ (the menu's Start)
//! hands the seats to the match's `controls::PadSlots`, Back leaves them as they were. Who is human doesn't change, so
//! the sim only sees which device presses for a seat.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use hst_data::iso::Iso;
use std::sync::Arc;

use super::super::widescreen;
use super::super::word_label::sheet;
pub(in crate::play) use super::super::word_label::label;
use super::super::controls::{PadSlots, Seat};
use super::{Art, Dev, Menu, POOL, Screen, Settings, Slot, Text, TickerClip, Ticker, slide};
use crate::Args;
use crate::audio::{MusicBanks, Sound, SoundBank};

/// The pause menu's Settings screens are up; `fresh` the frame they opened (its press is spent).
#[derive(Resource, Default)]
pub(in crate::play) struct PauseSettings {
    pub open: bool,
    fresh: bool,
    /// B254: the controller assignment is up, with the seats it opened on.
    seats: Option<[Option<Dev>; 4]>,
}

impl PauseSettings {
    pub(in crate::play) fn open(&mut self) {
        *self = PauseSettings { open: true, fresh: true, seats: None };
    }

    /// B254: the controller assignment.
    pub(in crate::play) fn open_seats(&mut self) {
        *self = PauseSettings { open: true, fresh: true, seats: Some([None; 4]) };
    }
}

/// B254: the match's seats 1P..4P as the assignment holds them: each human seat's device (none while its pad is out),
/// the others the computer's. A seat with both the keyboard and a pad (no `--pads`: 1P) shows its pad.
fn seats_now(humans: [bool; 4], slots: &PadSlots) -> [Option<Dev>; 4] {
    std::array::from_fn(|s| match slots.0[s] {
        _ if !humans[s] => None,
        Some(e) => Some(Dev::Pad(e)),
        None => (slots.keys() == Some(s)).then_some(Dev::Keys),
    })
}

/// B540: the human seats' devices the pause menu's assignment last handed the match, for the front-end it goes back to
/// (`Kept`'s menu, held in the match's way out since it started). Taken by `seats_back`.
static SWAPPED: std::sync::Mutex<Option<[Option<Option<Dev>>; 4]>> = std::sync::Mutex::new(None);

/// B540: back from a match whose seats were reassigned over the pause menu: the menu's seats follow (the computer's
/// stay as they were), and 1P's device, if it has one, drives the menus 1P alone drives (`Menu::p1`), as it would
/// have had it come in on the splash.
pub(super) fn seats_back(menu: &mut Menu) {
    let Some(seats) = SWAPPED.lock().unwrap().take() else { return };
    for (s, d) in seats.into_iter().enumerate() {
        if let Some(d) = d {
            menu.seats[s] = d;
        }
    }
    if let Some(d) = menu.seats[0] {
        menu.p1 = Some(d);
    }
}

/// B540: the assignment's seats as the front-end keeps them: each human seat's, none for the computer's.
fn swapped(menu: &Menu) -> [Option<Option<Dev>>; 4] {
    std::array::from_fn(|s| (!menu.bots[s]).then_some(menu.seats[s]))
}

/// B254: the assignment's seats into the match: each pad held by its seat (`pads`, the sorted list, gives `Seat`'s
/// index), the keyboard on its own; a seat left without a device takes none until assigned again.
fn apply(seats: [Option<Dev>; 4], pads: &[Entity], slots: &mut PadSlots) {
    slots.0 = seats.map(|d| match d {
        Some(Dev::Pad(e)) => Some(e),
        _ => None,
    });
    slots.1 = Some(seats.map(|d| match d {
        Some(Dev::Keys) => Some(Seat::Keys),
        Some(Dev::Pad(e)) => pads.iter().position(|x| *x == e).map(Seat::Pad),
        _ => None,
    }));
}

#[derive(Component)]
pub(in crate::play) struct Root;

/// This overlay's systems, after the pause menu's step (`menu.rs`).
#[derive(SystemSet, Clone, PartialEq, Eq, Hash, Debug)]
pub(in crate::play) struct Systems;

/// What the overlay draws on: the Settings screens' quads only need the font, the wall and our pill (9 quads
/// a pill since its rim, ring and fill are layered); B294's tabs and panels take up to ~800 (`options::tests::pages_fit`).
pub(super) const POOL_HERE: usize = 1400;



/// B293: the pause screen's venue caption, from `message0.dat` as the confirm screen names them: the match's court
/// (`--court`/`--stage` 1–11, none for a viewer court) and its umpire (`--umpire`), e.g. "Umpire Anna".
pub(in crate::play) fn venue(iso: &mut Iso, court: usize, umpire: u8) -> (String, String) {
    let msg = super::messages(&sheet(iso, "MENU/MENU00B.XB0", "/message0.dat"));
    let at = |i: usize| msg.get(i).map_or("", String::as_str).trim();
    let court = if (1..=11).contains(&court) { format!("{} {}", at(276 + 2 * (court - 1)), at(277 + 2 * (court - 1))).trim().to_string() } else { String::new() };
    (court, umpire_caption(at(669 + 2 * umpire as usize)))
}

/// "Umpire <name>" for the venue caption, from a disc name ("Umpire Anna") or a mod's own (B430: "Gloria", or one
/// that already says "Umpire 00"); nothing for no name.
pub(in crate::play) fn umpire_caption(name: &str) -> String {
    let name = name.trim().trim_start_matches("Umpire ").trim();
    if name.is_empty() { String::new() } else { format!("Umpire {name}") }
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
    let mut art = vec![solid, super::super::menu_art::image(&mut images, &font, "word")];
    // `Tex::Sheet(k)` is `art[2 + k]`: only the wall the Settings screens draw (Z21: and nothing else), the rest left empty
    for (k, (xb, name)) in super::SHEETS.iter().enumerate() {
        art.push(if k == super::WALL { super::super::menu_art::image(&mut images, &sheet(&mut iso, xb, &format!("/{name}.tm2")), name) } else { Handle::default() });
    }
    // `Tex::Pill` is the last sheet, as in the main menu
    art.push(super::super::menu_art::pill_layers(&mut images));
    let msg = super::messages(&sheet(&mut iso, "MENU/MENU00B.XB0", "/message0.dat"));
    let text = Text { msg, glyphs: super::glyphs(&font), grades: [[0; 5]; 14], style: [0; 14], hands: [1.0; 14], ranks: Vec::new(), mods: Vec::new() };
    let bank = SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd").map(Arc::new);
    commands.insert_resource(Art(art, bank, text));
    commands.insert_resource(Menu { screen: Screen::Display, ..default() });
    let full = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    // over the pause screen (`GlobalZIndex(10)`)
    // the quads only while it's up (`pool`)
    commands.spawn((full, GlobalZIndex(11), Visibility::Hidden, Root)).with_children(|p| {
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
fn opened(
    mut ps: ResMut<PauseSettings>,
    args: Res<Args>,
    mut menu: ResMut<Menu>,
    mut slide: ResMut<slide::Slide>,
    mut root: Query<&mut Visibility, With<Root>>,
    (game, pads, slots): (Option<Res<super::super::Game>>, Option<Res<super::super::Pads>>, Res<PadSlots>),
) {
    if ps.fresh {
        let settings = crate::store::read_to_string(Settings::path(&args.iso)).map_or(menu.settings, |t| Settings::parse(&t));
        if ps.seats.is_some() {
            let n = game.map_or(2, |g| g.players.len());
            let humans: [bool; 4] = std::array::from_fn(|s| s < n && pads.as_ref().is_some_and(|p| !p.autoplay && p.seats.map_or(s == 0 || p.connected > s, |h| h[s])));
            let seats = seats_now(humans, &slots);
            ps.seats = Some(seats);
            *menu = Menu { screen: Screen::Assign, settings, doubles: n == 4, seats, bots: humans.map(|h| !h), paused: true, ..default() };
            *slide = slide::Slide::default();
            for mut v in &mut root {
                v.set_if_neq(Visibility::Inherited);
            }
            return;
        }
        let screen = match std::env::var("HST_PAUSE_SETTINGS").as_deref() {
            Ok("display") => Screen::Display,
            Ok("graphics") => Screen::Graphics,
            Ok("sound") => Screen::Sound,
            Ok("controls") => Screen::Controls,
            _ => Screen::Display,
        };
        *menu = Menu { screen, settings, ..default() };
        *slide = slide::Slide::default();
    }
    for mut v in &mut root {
        v.set_if_neq(if ps.open { Visibility::Inherited } else { Visibility::Hidden });
    }
}

/// P39: the overlay's quads while it's up, none while it's hidden (UI layout walks hidden nodes too).
fn pool(mut commands: Commands, root: Query<(Entity, &Visibility), With<Root>>, slots: Query<(Entity, &ChildOf), With<Slot>>) {
    for (r, v) in &root {
        let have = slots.iter().filter(|(_, p)| p.parent() == r).map(|(e, _)| e);
        crate::ui_pool::keep(&mut commands, r, *v != Visibility::Hidden, have, POOL_HERE.min(POOL), |i| (Slot(i), crate::ui_pool::quad()));
    }
}

/// B294: the Settings pages lay out on the whole view, as the main menu's (`fit_grid`, less its live settings), at the
/// Menu Size while they're up; the match's HUD and legend (key hints, the pause dialog's) keep 100% otherwise.
fn fit(window: Query<&Window, With<PrimaryWindow>>, ps: Res<PauseSettings>, mut menu: ResMut<Menu>) {
    let Ok(w) = window.single() else { return };
    let k = if ps.open {
        menu.settings.apply_menu_scale(w)
    } else {
        widescreen::MENU_SCALE.store(1.0f32.to_bits(), std::sync::atomic::Ordering::Relaxed);
        1.0
    };
    let g = super::Grid::fit_at(widescreen::aspect(w), k);
    if menu.grid != g {
        menu.grid = g;
    }
}

use crate::widescreen::screen_point;
pub(in crate::play) use crate::widescreen::pointer_press;

/// The 640×448 point under the mouse, if it's over the window and not hidden (`pointer::MOUSE_IDLE`).
pub(in crate::play) fn cursor_point(window: &Query<&Window, With<PrimaryWindow>>) -> Option<Vec2> {
    if super::pointer::MOUSE_IDLE.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }
    let win = window.single().ok()?;
    win.cursor_position().map(|p| screen_point(win, p))
}

/// Back on the Settings screen leaves for the main menu's first screen: here, the pause menu. And the settings the
/// match can take live: widescreen and the court's music (the graphics and controls `super::step` sets already).
fn after(
    mut ps: ResMut<PauseSettings>,
    menu: Res<Menu>,
    (mut slots, gamepads): (ResMut<PadSlots>, Query<(Entity, &bevy::input::gamepad::Gamepad)>),
    mut args: ResMut<Args>,
    game: Option<Res<super::super::Game>>,
    sound: Option<Res<Sound>>,
    music: Option<ResMut<MusicBanks>>,
) {
    if !ps.open {
        return;
    }
    ps.fresh = false;
    // B254: the assignment's Start keeps the seats (if changed), its Back (to the mode screen) leaves them
    if let Some(was) = ps.seats {
        if menu.screen == Screen::Chars && menu.seats != was {
            let mut pads: Vec<_> = gamepads.iter().filter(|(_, g)| super::super::controls::readable(g)).map(|(e, _)| e).collect();
            pads.sort();
            apply(menu.seats, &pads, &mut slots);
            *SWAPPED.lock().unwrap() = Some(swapped(&menu));
        }
        if menu.screen != Screen::Assign {
            ps.open = false;
        }
        return;
    }
    let [_, _, match_music, ..] = menu.settings.0;
    widescreen::ASPECT.store(menu.settings.3, std::sync::atomic::Ordering::Relaxed);
    super::super::key_hints::ON.store(menu.settings.0[super::KEY_HINTS], std::sync::atomic::Ordering::Relaxed);
    if match_music != args.music
        && let (Some(game), Some(sound), Some(mut music)) = (game, sound, music)
    {
        args.music = match_music;
        if match_music {
            // `play.rs setup`'s pick: the court's track, else the default one
            let stage = game.stage as usize;
            let name = if (1..=11).contains(&stage) { format!("bgmg_{stage:02}") } else { "bgmg_14".into() };
            music.bgm = Iso::open(&args.iso).ok().and_then(|mut iso| SoundBank::music(&mut iso, "Court", &name)).map(|(b, m)| (Arc::new(b), m));
            // M8f6: a court mod's own
            crate::court_mod::bgm(&mut music.bgm);
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

pub(in crate::play) use super::super::netplay::{CHOICES, OnlineNotice};

#[derive(Component)]
struct NoticeSlot(usize);
#[derive(Component)]
struct NoticeRoot;
/// Three outlined lines of ~50 glyphs (9 quads each), the box, pills and hand.
const NOTICE_POOL: usize = 1600;

/// The window's quads and its pills' rects, centred on the 640×448 screen.
fn notice_layout(text: &Text, n: &OnlineNotice) -> (Vec<super::Quad>, Vec<[f32; 4]>) {
    use super::super::menu::{CHOSEN, PANEL, UNCHOSEN};
    use super::{INK, Tex, WHITE};
    let mut d = super::Draw { out: Vec::new(), text, said: String::new(), anchor: widescreen::Anchor::Centre, hits: Vec::new(), outside: None };
    if n.text.is_empty() {
        return (d.out, Vec::new());
    }
    let lines: Vec<&str> = n.text.lines().collect();
    let (x, w, max) = (120.0, 400.0, 368.0);
    let h = 18.0 + 26.0 * lines.len() as f32 + if n.ask { 40.0 * CHOICES.len() as f32 } else { 0.0 } + 14.0;
    let top = (224.0 - h / 2.0).round();
    d.anchor = widescreen::Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 64.0);
    d.anchor = widescreen::Anchor::Centre;
    d.box3([x, top, w, h], 16.0, [Some(INK), Some(WHITE), Some(PANEL)], 128.0);
    for (i, l) in lines.iter().enumerate() {
        // a long line (the score, a saved file's path) set smaller, down to 15 px, then squeezed across
        let size = (20.0 * max / d.width(l, 20.0).max(1.0)).clamp(15.0, 20.0);
        let y = top + 18.0 + 26.0 * i as f32 + (20.0 - size) / 2.0;
        d.outlined(|d| d.label(l, 320.0, y, size, max, WHITE));
    }
    let mut pills = Vec::new();
    if n.ask {
        let y0 = top + 18.0 + 26.0 * lines.len() as f32 + 8.0;
        for (k, word) in CHOICES.into_iter().enumerate() {
            let r = [176.0, y0 + 40.0 * k as f32, 288.0, 32.0];
            let on = k == n.sel;
            d.pill_at(r, [INK, WHITE, if on { CHOSEN } else { UNCHOSEN }], 128.0);
            // outlined, every one in full: a dimmed pill read as one that can't be chosen
            d.outlined(|d| d.label(word, 320.0, r[1] + 6.0, 20.0, 230.0, WHITE));
            pills.push(r);
        }
        // where the pause dialog's hand points (`menu.rs`: pill x − 60)
        d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [116.0, y0 + 40.0 * n.sel as f32, 64.0, 32.0], WHITE, 128.0);
    }
    (d.out, pills)
}

fn notice_setup(mut commands: Commands) {
    let full = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    // over the match's HUD, under the pause screen (which hides this root while it's up, as it did the plain text)
    // the quads only while a notice is up (`notice_pool`)
    commands.spawn((full, GlobalZIndex(9), NoticeRoot));
}

/// P39: the notice's quads while one is up (or `HST_NOTICE` asks for one), none otherwise.
fn notice_pool(mut commands: Commands, n: Res<OnlineNotice>, root: Query<Entity, With<NoticeRoot>>, slots: Query<Entity, With<NoticeSlot>>, mut env: Local<Option<bool>>) {
    let env = *env.get_or_insert_with(|| std::env::var_os("HST_NOTICE").is_some());
    for r in &root {
        crate::ui_pool::keep(&mut commands, r, env || !n.text.is_empty(), slots, NOTICE_POOL, |i| (NoticeSlot(i), crate::ui_pool::quad()));
    }
}

/// Draws [`OnlineNotice`]; the mouse over a pill chooses it, a click or tap on it answers (sound 0 on a move).
fn notice_draw(
    mut n: ResMut<OnlineNotice>,
    art: Option<Res<Art>>,
    cursor: Option<Res<super::cursor::Cursor>>,
    window: Query<&Window, With<PrimaryWindow>>,
    (mouse, touches): (Res<ButtonInput<MouseButton>>, Res<Touches>),
    sound: Option<Res<Sound>>,
    mut last: Local<(usize, bool)>,
    mut q: Query<(&NoticeSlot, &mut ImageNode, &mut Node, &mut Visibility)>,
) {
    use super::Tex;
    let Some(art) = art else { return };
    // `HST_NOTICE=<text, | between lines>` (with `HST_NOTICE_ASK=1` the question's pills) shows one, for `--shot`
    if n.text.is_empty()
        && let Ok(t) = std::env::var("HST_NOTICE")
    {
        (n.text, n.ask) = (t.replace('|', "\n"), std::env::var_os("HST_NOTICE_ASK").is_some());
    }
    let (quads, pills) = notice_layout(&art.2, &n);
    let on = |p: Vec2| pills.iter().position(|r| (r[0]..r[0] + r[2]).contains(&p.x) && (r[1]..r[1] + r[3]).contains(&p.y));
    if let Some(k) = cursor_point(&window).and_then(on) {
        n.sel = k;
    }
    if let Some(k) = pointer_press(&window, &mouse, &touches).0.and_then(on) {
        (n.sel, n.tapped) = (k, true);
    }
    if n.ask && last.1 && n.sel != last.0
        && let (Some(s), Some(b)) = (&sound, &art.1)
    {
        s.play_centre(b, hst_sim::sound::Play { slot: 9, program: 0, key: 0, volume: 0x80, speed: 1.0 });
    }
    *last = (n.sel, n.ask);
    let share = widescreen::window_share(&window);
    let aspect = widescreen::window_aspect(&window);
    for (NoticeSlot(i), mut img, mut node, mut vis) in &mut q {
        let Some(quad) = quads.get(*i) else {
            vis.set_if_neq(Visibility::Hidden);
            continue;
        };
        let three_d = cursor.as_ref().filter(|_| quad.tex == Tex::Cursor);
        let image = match (three_d, quad.tex) {
            (Some(c), _) => c.image.clone(),
            (_, Tex::Solid) => art.0[0].clone(),
            (_, Tex::Font) => art.0[1].clone(),
            (_, Tex::Pill) => art.0.last().expect("pill").clone(),
            _ => {
                vis.set_if_neq(Visibility::Hidden);
                continue;
            }
        };
        let [u, v, w, h] = quad.src;
        let rect = (quad.tex != Tex::Solid && three_d.is_none()).then(|| Rect::new(u, v, u + w, v + h));
        let [r, g, b] = quad.rgb.map(|c| c / 128.0);
        crate::ui_pool::paint(&mut img, image, rect, Color::srgba(r, g, b, quad.alpha / 128.0));
        let [mut x, mut y, mut w, mut h] = quad.dst;
        if three_d.is_some() {
            // the hand's image covers `PAD` times its quad, around the same centre (B146)
            let pad = super::cursor::PAD;
            (x, y, w, h) = (x - w * (pad - 1.0) / 2.0, y - h * (pad - 1.0) / 2.0, w * pad, h * pad);
        }
        let (left, width) = widescreen::place(x, w, share, quad.anchor);
        let (top, height) = widescreen::vplace(y, h, aspect, quad.anchor);
        widescreen::put(&mut node, left, top, width, height);
        vis.set_if_neq(Visibility::Inherited);
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
        .init_resource::<OnlineNotice>()
        .add_systems(PostStartup, (setup, notice_setup))
        .add_systems(Update, (notice_pool, notice_draw).chain().after(Systems))
        .add_systems(
            Update,
            (opened, pool, fit, super::pointer::pointer.run_if(live), super::step.run_if(live), after, (super::draw, super::ticker_text).run_if(open))
                .chain()
                .in_set(Systems),
        )
        // M14: the Settings screens' legend, as the main menu's
        .add_systems(Update, super::legend::fill.run_if(open.and_then(resource_exists::<super::legend::Legend>)).after(Systems));
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

    /// B430: the venue caption names an umpire mod by its own name (with or without "Umpire " in it), like a disc one.
    #[test]
    fn umpire_caption_names_a_mod_by_its_own_name() {
        assert_eq!(umpire_caption("Umpire Anna"), "Umpire Anna");
        assert_eq!(umpire_caption("Gloria"), "Umpire Gloria");
        assert_eq!(umpire_caption("Umpire 00"), "Umpire 00");
        assert_eq!(umpire_caption("  "), "");
    }

    /// B254: doubles, 1P on the keyboard, 3P on pad a, 2P and 4P the computer's. 1P's → passes the computer's seat
    /// (its device to 3P, pad a now 1P), △ adds no computer player, the keyboard takes itself off, a new pad b's ✕
    /// takes the free human seat, the keyboard's ✕ hands it to the match's slots, which keep it through `update`.
    /// B437: the keyboard drives the screen until it leaves (its own move pending), so it can keep switching.
    #[test]
    fn reassign_in_a_match() {
        let [a, b] = [5, 7].map(|n| Entity::from_raw_u32(n).unwrap());
        let mut slots = PadSlots([None, None, Some(a), None], Some([Some(Seat::Keys), None, Some(Seat::Pad(0)), None]));
        let humans = [true, false, true, false];
        let seats = seats_now(humans, &slots);
        assert_eq!(seats, [Some(Dev::Keys), None, Some(Dev::Pad(a)), None]);
        let mut m = Menu { screen: Screen::Assign, doubles: true, seats, bots: humans.map(|h| !h), paused: true, ..default() };
        m.step(Dev::Keys, Press { right: true, ..default() });
        assert_eq!((m.seats, m.bots, m.sel), ([Some(Dev::Pad(a)), None, Some(Dev::Keys), None], [false, true, false, true], 2));
        assert_eq!((m.lead(), m.lead_pending()), (Some(Dev::Keys), Some(Some(2))));
        // still the keyboard's: back to 1P and over again; pad a, moved onto 1P, neither drives nor moves nor leaves
        m.step(Dev::Keys, Press { right: true, ..default() });
        assert_eq!((m.seats, m.lead_pending()), ([Some(Dev::Keys), None, Some(Dev::Pad(a)), None], None));
        m.step(Dev::Keys, Press { left: true, ..default() });
        assert_eq!(m.seats, [Some(Dev::Pad(a)), None, Some(Dev::Keys), None]);
        for k in [Press { right: true, ..default() }, Press { back: true, ..default() }, Press { ok: true, ..default() }] {
            m.step(Dev::Pad(a), k);
        }
        assert_eq!((m.screen, m.seats), (Screen::Assign, [Some(Dev::Pad(a)), None, Some(Dev::Keys), None]));
        m.sel = 1;
        assert_eq!((m.step(Dev::Keys, Press { card: true, ..default() }), m.bots[1]), (vec![], true));
        // no free human seat yet: b's ✕ does nothing; the keyboard takes itself off (□), then b takes 3P
        m.step(Dev::Pad(b), Press { ok: true, ..default() });
        assert!(!m.seats.contains(&Some(Dev::Pad(b))));
        m.sel = 2;
        m.step(Dev::Keys, Press { page: true, ..default() });
        assert_eq!(m.lead_pending(), Some(None));
        m.step(Dev::Pad(b), Press { ok: true, ..default() });
        assert_eq!(m.seats, [Some(Dev::Pad(a)), None, Some(Dev::Pad(b)), None]);
        // 1P alone may take itself off too (no moving up over the computer's seats)
        assert!(m.droppable(0) && m.droppable(2));
        m.step(Dev::Keys, Press { ok: true, ..default() });
        assert_eq!(m.screen, Screen::Chars);
        assert_eq!(m.lead(), Some(Dev::Pad(a)), "left: pad a is 1P");
        apply(m.seats, &[a, b], &mut slots);
        slots.update(&[a, b]);
        assert_eq!((slots.0, slots.keys()), ([Some(a), None, Some(b), None], None));
        // no `--pads`: 1P's pad stands for the keyboard beside it, a missing pad leaves its human seat empty
        assert_eq!(seats_now([true, true, false, false], &PadSlots([Some(b), None, None, None], None)), [Some(Dev::Pad(b)), None, None, None]);
    }

    /// B540: singles, 1P on the keyboard, 2P on pad a. Over the match they trade seats; back on the front-end pad a
    /// is 1P and drives the main screen, the keyboard sits in 2P, and another pad (b, never seated) still can't.
    #[test]
    fn swapped_seats_drive_the_front_end_after_the_match() {
        let [a, b] = [5, 7].map(|n| Entity::from_raw_u32(n).unwrap());
        let mut front = Menu { seats: [Some(Dev::Keys), Some(Dev::Pad(a)), None, None], p1: Some(Dev::Keys), ..default() };
        let humans = [true, true, false, false];
        let slots = PadSlots([None, Some(a), None, None], Some([Some(Seat::Keys), Some(Seat::Pad(0)), None, None]));
        let mut m = Menu { screen: Screen::Assign, seats: seats_now(humans, &slots), bots: humans.map(|h| !h), paused: true, ..default() };
        m.step(Dev::Keys, Press { right: true, ..default() });
        m.step(Dev::Keys, Press { ok: true, ..default() });
        assert_eq!((m.screen, m.seats), (Screen::Chars, [Some(Dev::Pad(a)), Some(Dev::Keys), None, None]));
        *SWAPPED.lock().unwrap() = Some(swapped(&m));
        front.after_match();
        seats_back(&mut front);
        assert_eq!((front.seats, front.p1, front.lead()), ([Some(Dev::Pad(a)), Some(Dev::Keys), None, None], Some(Dev::Pad(a)), Some(Dev::Pad(a))));
        let local = front.main_at(super::super::home::Row::Local);
        front.sel = local + 1;
        front.step(Dev::Pad(b), Press { up: true, ..default() });
        assert_eq!(front.sel, local + 1, "pad b isn't 1P");
        front.step(Dev::Pad(a), Press { up: true, ..default() });
        assert_eq!(front.sel, local, "pad a, now 1P, drives the main screen");
        // taken: the next match back with no reassignment leaves the seats alone
        seats_back(&mut front);
        assert!(SWAPPED.lock().unwrap().is_none());
    }

    /// A row changes and saves; Back on a Settings tab goes to `Screen::Main`, which `after` takes as closing.
    #[test]
    fn back_returns_to_pause() {
        let aspect = super::super::options::page(Screen::Display).iter().position(|o| *o == super::super::options::Opt::Aspect).unwrap();
        let mut m = Menu { screen: Screen::Display, sel: aspect, ..default() };
        // B498: ✕ opens Aspect Ratio's list, ↓ moves to the next, ✕ picks it
        m.step(super::super::Dev::Keys, Press { ok: true, ..default() });
        m.step(super::super::Dev::Keys, Press { down: true, ..default() });
        let fx = m.step(super::super::Dev::Keys, Press { ok: true, ..default() });
        assert!(m.settings.3 == 1 && fx.contains(&super::super::Effect::SaveSettings));
        m.step(super::super::Dev::Keys, Press { back: true, ..default() });
        assert_eq!(m.screen, Screen::Main);
    }

    /// B139: the overlay's systems run in an app with only what a match has (no main menu): open Settings, toggle
    /// Menu Music (not the Aspect Ratio: `after` stores that in a global) on the Sound tab, Back to the
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
        let was = app.world().resource::<Menu>().settings.0[1];
        // the Sound tab (2), then its Menu Music row (5)
        app.world_mut().resource_mut::<super::super::pointer::Pending>().0.push((super::super::pointer::Act::Tab(2), true));
        app.update();
        // B326: a tap picks the row, a tap on its value's right half changes it, a tap on the row keeps it
        for k in [super::super::pointer::OK, Press { right: true, ..default() }, super::super::pointer::OK] {
            app.world_mut().resource_mut::<super::super::pointer::Pending>().0.push((super::super::pointer::Act::Row(5, k), true));
            app.update();
        }
        assert_eq!(app.world().resource::<Menu>().settings.0[1], !was, "Menu Music didn't change");
        assert!(std::fs::read_to_string(dir.join("settings.txt")).is_ok(), "settings not saved");
        app.world_mut().resource_mut::<super::super::pointer::Pending>().0.push((super::super::pointer::press(|k| k.back = true), true));
        app.update();
        assert!(!app.world().resource::<PauseSettings>().open, "Back didn't return to the pause menu");
        app.update();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
