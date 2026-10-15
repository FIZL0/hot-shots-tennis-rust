//! B290: the Settings screen's pages (Display, Graphics, Sound, Controls, Online & Mods); every page but Controls is a
//! column of options, each a row with its value (or volume slider) between ◀ ▶ arrows.
//! B294 (user's mockup, 2026-10-14): the pages are tabs across the title band (L1 / R1), the rows compact in a list
//! panel, a side panel with the chosen option's name, description and choices; no description bar (the legend
//! strip's keys only); laid out on the whole view at any aspect (the select's [`Grid`] says how far it reaches).

use super::super::widescreen::ASPECTS;
use super::{BROWN, CHAT, VIBRATION, CELL_FILL, CELL_ON, CREAM, Draw, Effect, Grid, INK, KEY_HINTS, MENU_SIZES, Menu, Press, RENDERERS, Screen, Settings, Tex, UPSCALED_DOWNLOADS, WHITE, controls, pointer};
use crate::mods_online::Ask;
use crate::graphics::{FPS, Graphics, MSAA, SCALES, SHADOWS};
use bevy::prelude::default;

/// One option row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Opt {
    Display,
    Aspect,
    /// The menus' size (`widescreen::MENU_SCALE`).
    MenuSize,
    ShowFps,
    /// B301: the match's keyboard hints.
    KeyHints,
    Preset,
    Scale,
    Shadows,
    Msaa,
    Fps,
    Vsync,
    Original,
    Upscaled,
    /// B345: leave off-screen characters and effects undrawn (`cull.rs`).
    Cull,
    /// `Volumes`' slider `i`, 0..=10.
    Volume(usize),
    MenuMusic,
    MatchMusic,
    Downloads,
    /// B339: how many store downloads run at once.
    AtOnce,
    /// B289: what to do with an online lobby's mods.
    OnlineMods,
    /// M20: online chat shown.
    Chat,
    /// B273: the browser build's renderer.
    Renderer,
    /// B118: the on-screen pad buzzes on each press.
    Vibration,
    /// B343: the speed-up switches (`Graphics::fewer_shadows`, `batch_crowd`; B342: `merge_meshes`).
    FewerShadows,
    BatchCrowd,
    MergeMeshes,
}
use Opt::*;

/// The Settings tabs, in order, and the pages they open; Mobile (B118) only on a touch screen (`tabs_shown`).
pub(super) const TABS: [(&str, Screen); 6] = [
    ("Display", Screen::Display),
    ("Graphics", Screen::Graphics),
    ("Sound", Screen::Sound),
    ("Controls", Screen::Controls),
    ("Online & Mods", Screen::ModSettings),
    ("Mobile", Screen::Mobile),
];
/// The tabs on show: Mobile's phone-only options on a touch screen.
pub(super) fn tabs_shown() -> &'static [(&'static str, Screen)] {
    if super::touch_page() { &TABS } else { &TABS[..5] }
}
/// `page`'s tab.
pub(super) fn tab(page: Screen) -> usize {
    TABS.iter().position(|(_, s)| *s == page).unwrap_or(0)
}

/// The browser build stays in its page (`graphics::KEEP_WINDOWED`), so it has no Display mode row; it has a
/// Renderer instead (B273).
const DISPLAY: &[Opt] = if cfg!(target_arch = "wasm32") { &[Renderer, Aspect, MenuSize, ShowFps, KeyHints, FewerShadows, BatchCrowd, MergeMeshes] } else { &[Display, Aspect, MenuSize, ShowFps, KeyHints, FewerShadows, BatchCrowd, MergeMeshes] };
/// What a preset sets, then the textures.
const GRAPHICS: &[Opt] = &[Preset, Scale, Shadows, Msaa, Fps, Vsync, Original, Upscaled, Cull];
const SOUND: &[Opt] = &[Volume(0), Volume(1), Volume(2), Volume(3), MenuMusic, MatchMusic];
const MODS: &[Opt] = &[Downloads, AtOnce, OnlineMods, Chat];
/// B118: phone-only options (later: control size and opacity).
const MOBILE: &[Opt] = &[Vibration];
/// B289: Online Mod Downloads' values.
const ASK_VALUES: [(Ask, &str); 3] = [(Ask::Each, "Ask"), (Ask::Always, "Always"), (Ask::Never, "Never")];

/// Page `s`'s options (none on a screen that isn't an options page).
pub(super) fn page(s: Screen) -> &'static [Opt] {
    match s {
        Screen::Display => DISPLAY,
        Screen::Graphics => GRAPHICS,
        Screen::Sound => SOUND,
        Screen::ModSettings => MODS,
        Screen::Mobile => MOBILE,
        _ => &[],
    }
}

/// The options the browser build keeps off and locked (B227: Upscaled Textures, `textures::set`, and Upscaled
/// Downloads, `modstore::install`).
pub(super) fn locked(o: Opt) -> bool {
    cfg!(target_arch = "wasm32") && matches!(o, Upscaled | Downloads)
}

/// Whether ✕ changes `o` too (a slider only takes ◀ ▶).
pub(super) fn ok_changes(o: Opt) -> bool {
    !matches!(o, Volume(_)) && !locked(o)
}

fn label(o: Opt) -> &'static str {
    match o {
        Display => "Display",
        Aspect => "Aspect Ratio",
        MenuSize => "Menu Size",
        ShowFps => "Show FPS",
        KeyHints => "Key Hints",
        Preset => "Preset",
        Scale => "Resolution",
        Shadows => "Shadows",
        Msaa => "Anti-aliasing",
        Fps => "Frame Limit",
        Vsync => "VSync",
        Original => "Original Mode",
        Upscaled => "Upscaled Textures",
        Cull => "Skip Off-Screen",
        Volume(i) => ["Master", "Music", "Sound Effects", "Voices"][i],
        MenuMusic => "Menu Music",
        MatchMusic => "Match Music",
        Downloads => "Upscaled Downloads",
        AtOnce => "Downloads at Once",
        OnlineMods => "Online Mod Downloads",
        Chat => "Chat",
        Renderer => "Renderer",
        Vibration => "Vibration",
        FewerShadows => "Fewer Shadows",
        BatchCrowd => "Batch Crowd",
        MergeMeshes => "Merge Court Meshes",
    }
}

fn about(o: Opt) -> &'static str {
    match o {
        Display => "Borderless fullscreen on the current monitor, or a window.",
        Aspect => "Auto fills the window; a fixed ratio shows the whole picture at that shape, black bars around it.",
        MenuSize => "Makes every menu smaller or bigger. The whole layout scales together, never stretched; the extra room goes around it.",
        ShowFps => "Show the frame rate in the bottom-left corner, in the menus and in a match. Detailed: where each frame's time goes (game, simulation, rendering, waiting on the GPU/browser), meshes drawn and the renderer.",
        KeyHints => "Show the keyboard's keys at the bottom during a match while you play on the keyboard.",
        Preset => "Default draws at full quality; Steam Deck, Low and Phone trade detail for frame rate.",
        Scale => "Draw the court at this share of the window's resolution, scaled up to fill it (the HUD stays sharp).",
        Shadows => "Off, or the shadow map's detail (Low: one cascade, and only what shades the court casts).",
        Msaa => "Smooth the court's jagged edges (MSAA): off, 2x or 4x samples per pixel.",
        Fps => "The most frames drawn a second (the game itself always runs at 60 Hz).",
        Vsync => "Wait for the display between frames: no tearing, at most its refresh rate.",
        MergeMeshes => "Draw all copies of each court prop as one mesh (same picture). Off unless it runs faster on your device; turning it on applies from the next match.",
        Original => "Draw at the PS2's 640x448, 4:3, with the disc's textures, scaled up unfiltered.",
        Upscaled | Downloads if locked(o) => "Upscaled textures aren't available in the browser.",
        Upscaled => "Use the upscaled textures in mods/texture-replacements.",
        Cull => "Don't draw players, figures and effects that are out of view (or their shadows out of the light's). Looks the same; turn it off if it makes this device slower.",
        Volume(0) => "The volume of everything.",
        Volume(1) => "The menu and match music and the jingles.",
        Volume(2) => "The court, the crowd, the weather and the menu sounds.",
        Volume(_) => "The players' and the umpire's voices.",
        MenuMusic => "Play music in the menus.",
        MatchMusic => "Play each court's music during a match.",
        Downloads => "Download mods' upscaled textures from the mod store (most of a mod's size).",
        AtOnce => "How many mods the mod store downloads side by side (Download all, or several installs). Auto: 3 on a computer, 2 in a browser, 1 on a phone; a browser keeps each one in memory until it's saved, so it allows fewer.",
        OnlineMods => "When an online lobby has a mod you don't have: ask each time, always download it, or never (a stand-in plays it).",
        Chat => "Show the online lobby's and match's text chat (T, or R2/L2 on a pad), or hide it.",
        Renderer => "Auto: WebGPU when the browser has it, else WebGL2. The page reloads into it a moment after a change.",
        Vibration => vibration_about(),
        FewerShadows => "At Low shadows, only the players, the umpire and what shades the court cast: fewer draws. Off: Low casts as many as High.",
        BatchCrowd => "Draw identical court creatures together in one draw, from the next match. Off by default: no measurable gain.",
    }
}

/// B329: the Vibration row's text, by what this browser can do (`web/touch.js`).
fn vibration_about() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    let can = crate::web::vibration();
    #[cfg(not(target_arch = "wasm32"))]
    let can = "vibrate";
    match &*can {
        "vibrate" => "Buzz briefly on each press of the on-screen controls. Android: not while the phone's ringer is on Silent.",
        "ios" => "iPhone: Safari has no vibration, so each press gives the light haptic tick of iOS 18 and later instead (nothing on older iOS).",
        _ => "This browser can't vibrate, so this does nothing here.",
    }
}

/// A Menu Size value's word (`MENU_SIZES`: 0 Auto, else tenths).
fn menu_size_name(t: u8) -> String {
    if t == 0 { "Auto".into() } else { format!("{}%", t as u32 * 10) }
}

fn fps_name(fps: u32) -> String {
    if fps == 0 { "Uncapped".into() } else { format!("{fps} fps") }
}

/// B316: the words of the rows whose values are small indices, shared by [`choices`] and [`Settings::value`].
const SHOW_FPS_NAMES: [&str; 3] = ["Off", "On", "Detailed"];
const SHADOW_NAMES: [&str; 3] = ["Off", "Low", "High"];
const MSAA_NAMES: [&str; 3] = ["Off", "2x", "4x"];

/// B294: every value `o` can take, in order, as [`Settings::value`] words it (the side panel's choices).
fn choices(o: Opt) -> Vec<String> {
    let words = |v: &[&str]| v.iter().map(|w| w.to_string()).collect();
    match o {
        _ if locked(o) => words(&["Off"]),
        Display => words(&["Fullscreen", "Window"]),
        Aspect => words(&ASPECTS),
        MenuSize => MENU_SIZES.iter().map(|&t| menu_size_name(t)).collect(),
        Preset => Graphics::PRESETS.iter().map(|(n, _)| n.to_string()).chain(["Custom".into()]).collect(),
        Scale => SCALES.iter().map(|s| format!("{s} %")).collect(),
        Shadows => words(&SHADOW_NAMES),
        Msaa => words(&MSAA_NAMES),
        ShowFps => words(&SHOW_FPS_NAMES),
        Renderer => RENDERERS.iter().map(|(_, n)| n.to_string()).collect(),
        Fps => FPS.map(fps_name).to_vec(),
        Volume(_) => (0..=10).map(|v| v.to_string()).collect(),
        OnlineMods => ASK_VALUES.iter().map(|(_, n)| n.to_string()).collect(),
        AtOnce => (0..=super::max_downloads()).map(at_once_name).collect(),
        _ => words(&["Off", "On"]),
    }
}

/// B339: a Downloads at Once value's word (0 Auto).
fn at_once_name(n: usize) -> String {
    if n == 0 { "Auto".into() } else { n.to_string() }
}

fn on_off(on: bool) -> String {
    if on { "On" } else { "Off" }.into()
}

/// The next (`d` = 1) or previous (−1) of `list` after `cur` (the first if `cur` isn't in it).
fn cycle<T: PartialEq + Copy>(list: &[T], cur: T, d: isize) -> T {
    let i = list.iter().position(|x| *x == cur).map_or(0, |i| (i as isize + d).rem_euclid(list.len() as isize) as usize);
    list[i]
}

impl Settings {
    /// Change `o` one step in direction `d`.
    pub(super) fn step(&mut self, o: Opt, d: isize) {
        let g = &mut self.1;
        match o {
            Display => g.windowed = !g.windowed,
            Aspect => self.3 = (self.3 as isize + d).rem_euclid(ASPECTS.len() as isize) as u8,
            MenuSize => self.7 = cycle(&MENU_SIZES, self.7, d),
            ShowFps => g.show_fps = (g.show_fps as isize + d).rem_euclid(3) as u8,
            KeyHints => self.0[KEY_HINTS] = !self.0[KEY_HINTS],
            Preset => {
                let names: Vec<_> = Graphics::PRESETS.iter().map(|(n, _)| *n).collect();
                let n = cycle(&names, g.preset(), d);
                *g = g.keep(Graphics::PRESETS.iter().find(|(m, _)| *m == n).unwrap().1);
            }
            Scale => g.scale = cycle(&SCALES, g.scale, d),
            Shadows => g.shadows = cycle(&SHADOWS, g.shadows, d),
            Msaa => g.msaa = cycle(&MSAA, g.msaa, d),
            Fps => g.fps = cycle(&FPS, g.fps, d),
            Vsync => g.vsync = !g.vsync,
            Original => g.original = !g.original,
            Cull => g.cull = !g.cull,
            // Upscaled Downloads follows Upscaled Textures (B227)
            Upscaled => {
                self.0[0] = !self.0[0];
                self.0[UPSCALED_DOWNLOADS] = self.0[0];
            }
            Volume(i) => {
                let v = &mut self.2 .0[i];
                *v = (*v as isize + d).clamp(0, 10) as _;
            }
            MenuMusic => self.0[1] = !self.0[1],
            MatchMusic => self.0[2] = !self.0[2],
            Downloads => self.0[UPSCALED_DOWNLOADS] = !self.0[UPSCALED_DOWNLOADS],
            AtOnce => self.8 = (self.8 as isize + d).rem_euclid(super::max_downloads() as isize + 1) as u8,
            OnlineMods => self.4 = cycle(&ASK_VALUES.map(|(a, _)| a), self.4, d),
            Chat => {
                self.0[CHAT] = !self.0[CHAT];
                crate::chat::HIDDEN.store(!self.0[CHAT], std::sync::atomic::Ordering::Relaxed);
            }
            Renderer => self.5 = (self.5 as isize + d).rem_euclid(RENDERERS.len() as isize) as u8,
            Vibration => self.0[VIBRATION] = !self.0[VIBRATION],
            FewerShadows => g.fewer_shadows = !g.fewer_shadows,
            BatchCrowd => g.batch_crowd = !g.batch_crowd,
            MergeMeshes => g.merge_meshes = !g.merge_meshes,
        }
    }

    /// The value shown beside `o` (a slider's is its number).
    pub(super) fn value(&self, o: Opt) -> String {
        let g = &self.1;
        match o {
            _ if locked(o) => "Off".into(),
            Display => if g.windowed { "Window" } else { "Fullscreen" }.into(),
            Aspect => ASPECTS[self.3 as usize].into(),
            MenuSize => menu_size_name(self.7),
            ShowFps => SHOW_FPS_NAMES[g.show_fps.min(2) as usize].into(),
            KeyHints => on_off(self.0[KEY_HINTS]),
            Preset => g.preset().into(),
            Scale => format!("{} %", g.scale),
            Shadows => SHADOW_NAMES[g.shadows as usize].into(),
            Msaa => MSAA_NAMES[g.msaa.ilog2().min(2) as usize].into(),
            Fps => fps_name(g.fps),
            Vsync => on_off(g.vsync),
            Original => on_off(g.original),
            MergeMeshes => on_off(g.merge_meshes),
            Cull => on_off(g.cull),
            Upscaled => on_off(self.0[0]),
            Volume(i) => self.2 .0[i].to_string(),
            MenuMusic => on_off(self.0[1]),
            MatchMusic => on_off(self.0[2]),
            Downloads => on_off(self.0[UPSCALED_DOWNLOADS]),
            AtOnce => at_once_name(self.8 as usize),
            OnlineMods => ASK_VALUES.iter().find(|(a, _)| *a == self.4).unwrap().1.into(),
            Chat => on_off(self.0[CHAT]),
            Renderer => RENDERERS[self.5 as usize].1.into(),
            Vibration => on_off(self.0[VIBRATION]),
            FewerShadows => on_off(g.fewer_shadows),
            BatchCrowd => on_off(g.batch_crowd),
        }
    }
}

impl Menu {
    /// An options page: ◀ ▶ (and ✕, but on a slider) change the row's option and save; L1 / R1 and ○ as [`Self::step_tabs`].
    pub(super) fn step_page(&mut self, k: Press, out: &mut Vec<Effect>) {
        if let Some(&o) = page(self.screen).get(self.sel)
            && !locked(o)
            && (k.left || k.right || (k.ok && ok_changes(o)))
        {
            self.settings.step(o, if k.left { -1 } else { 1 });
            out.extend([Effect::Sound(0), Effect::SaveSettings]);
            return;
        }
        self.step_tabs(k, out);
    }

    /// Any Settings page: L1 / R1 open the tab before / after it (wrapping), ○ goes back to the main menu's
    /// Settings row (closes the pause overlay).
    pub(super) fn step_tabs(&mut self, k: Press, out: &mut Vec<Effect>) {
        if k.prev || k.next {
            let tabs = tabs_shown();
            let n = tabs.len();
            self.go(tabs[(tab(self.screen) + if k.next { 1 } else { n - 1 }) % n].1, out);
        } else if k.back {
            self.sel = self.main_at(super::home::Row::Settings);
            self.screen = Screen::Main;
            out.push(Effect::Sound(1));
        }
    }
}

/// Words' purple-grey (the mockup's #7a4a5c): footnotes and headings.
pub(super) const MUTED: [f32; 3] = [61.0, 37.0, 46.0];

/// B294: the Settings pages' list (of `n` rows) and side panel on the view `g` fits (the side panel under the list in a portrait
/// window), from under the tabs down to the legend strip.
pub(super) fn boxes(g: &Grid, n: usize) -> ([f32; 4], [f32; 4]) {
    let [l, y, w, _] = g.strip;
    let bottom = g.detail[1] + g.detail[3] + 32.0;
    if g.detail[0] <= g.strip[0] {
        // portrait: the list as tall as the page's `n` rows want, the panel the rest
        let h = 30.0 + 32.0 * n as f32;
        ([l, y, w, h], [l, y + h + 8.0, w, bottom - y - h - 8.0])
    } else {
        let dw = 220.0f32.min(w * 0.4);
        ([l, y, w - dw - 8.0, bottom - y], [l + w - dw, y, dw, bottom - y])
    }
}

/// Row `i` of `n` in the list `list`: 36 apart, closer when they'd run past the footnote.
pub(super) fn row(list: [f32; 4], n: usize, i: usize) -> [f32; 4] {
    let pitch = ((list[3] - 30.0) / n as f32).min(36.0);
    [list[0] + 8.0, list[1] + 8.0 + pitch * i as f32, list[2] - 16.0, pitch - 4.0]
}

/// The tabs across the title band, right of the title's tab (shrunk to fit a narrow window); a tap opens one.
fn tabs(d: &mut Draw, m: &Menu) {
    let tabs = tabs_shown();
    let words: Vec<&str> = tabs.iter().map(|t| t.0).collect();
    tab_row(d, m, 1, &words, tabs.iter().position(|t| t.1 == m.screen));
}

/// Pills `words` across the title band, right of title `title`'s tab (shrunk to fit a narrow window), `on` dark; a
/// tap on one is `Act::Tab(i)` (Settings, the Mods page).
pub(super) fn tab_row(d: &mut Draw, m: &Menu, title: usize, words: &[&str], on: Option<usize>) {
    // right of the title's tab (`Draw::frame`), to the window's right edge (the strip spans the view less 12 a side)
    let x0 = d.width(super::TITLES[title].0, 30.0) + 88.0;
    let room = m.grid.strip[2] + 24.0 - x0 - 8.0;
    let ws: Vec<f32> = words.iter().map(|l| d.width(l, 15.0) + 22.0).collect();
    let k = (room / (ws.iter().sum::<f32>() + 6.0 * (words.len() - 1) as f32)).min(1.0);
    // in the centred screen's units from the view's top-left (`Span` would stretch a tab past 640 to the edge)
    let (mut x, top) = (x0 + m.grid.strip[0] - 12.0, m.grid.strip[1] - 46.0);
    for (i, (words, w)) in words.iter().zip(&ws).enumerate() {
        let on = on == Some(i);
        let b = [x, top + 3.0, w * k, 24.0];
        d.pill_at(b, [INK, if on { INK } else { [90.0; 3] }, if on { BROWN } else { CELL_FILL }], 128.0);
        d.label(words, x + w * k / 2.0, top + 7.5, 15.0, (w - 18.0) * k, if on { WHITE } else { BROWN });
        d.hit(b, pointer::Act::Tab(i));
        x += (w + 6.0) * k;
    }
}

/// `s` broken at spaces to lines `w` wide at `size` from (x, y), `gap` px apart; the y under its last line.
pub(super) fn para(d: &mut Draw, s: &str, x: f32, y: f32, w: f32, size: f32, gap: f32, rgb: [f32; 3]) -> f32 {
    let mut lines: Vec<String> = Vec::new();
    for word in s.split_whitespace() {
        match lines.last_mut() {
            Some(l) if d.width(&format!("{l} {word}"), size) <= w => *l = format!("{l} {word}"),
            _ => lines.push(word.into()),
        }
    }
    for (i, l) in lines.iter().enumerate() {
        d.text(l, x, y + gap * i as f32, size, rgb);
    }
    y + gap * lines.len() as f32
}

/// The frame, tabs, list panel and side panel every page shares; the list box.
fn chrome(d: &mut Draw, m: &Menu, n: usize) -> ([f32; 4], [f32; 4]) {
    d.frame(1, 6);
    tabs(d, m);
    let (list, side) = boxes(&m.grid, n);
    for b in [list, side] {
        d.box3(b, 14.0, [Some(INK), Some(WHITE), Some(CREAM)], 128.0);
    }
    (list, side)
}

/// The chosen row's highlight and the hand beside it.
pub(super) fn chosen(d: &mut Draw, r: [f32; 4], hand_x: f32) {
    d.box3(r, 10.0, [Some(INK), None, Some(CELL_ON)], 128.0);
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [r[0] - 16.0 + hand_x, r[1] + r[3] / 2.0 - 9.0, 36.0, 18.0], WHITE, 128.0);
}

/// The side panel: a name, what it does, its choices as chips (the current one dark) and a footnote.
fn side_panel(d: &mut Draw, side: [f32; 4], name: &str, about: &str, heading: &str, chips: &[String], now: Option<&str>, note: &str) {
    let [x, y, w, _] = side;
    let (x, w) = (x + 14.0, w - 28.0);
    let mut y = para(d, name, x, y + 12.0, w, 22.0, 26.0, BROWN) + 4.0;
    y = para(d, about, x, y, w, 14.0, 18.0, BROWN) + 10.0;
    if !chips.is_empty() {
        d.text(heading, x, y, 12.0, MUTED);
        y += 18.0;
        let mut cx = x;
        for c in chips {
            let cw = d.width(c, 13.0) + 16.0;
            if cx + cw > x + w && cx > x {
                (cx, y) = (x, y + 24.0);
            }
            let on = now == Some(c.as_str());
            d.pill_at([cx, y, cw, 20.0], [INK, if on { INK } else { [90.0; 3] }, if on { BROWN } else { CELL_FILL }], 128.0);
            d.centred(c, cx + cw / 2.0, y + 3.5, 13.0, if on { WHITE } else { BROWN });
            cx += cw + 5.0;
        }
        y += 32.0;
    }
    para(d, note, x, y, w, 12.0, 16.0, MUTED);
}

/// An options page: the rows in the list, each label on the left and its value between ◀ ▶ on the right (a tap on
/// the value's left half steps back, on its right half on, B221), a volume as a bar beside its number; the chosen
/// one's name, description and choices in the side panel.
pub(super) fn page_layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    let opts = page(m.screen);
    let (list, side) = chrome(d, m, opts.len());
    for (i, &o) in opts.iter().enumerate() {
        let r @ [x, y, w, h] = row(list, opts.len(), i);
        let on = i == m.sel;
        if on {
            chosen(d, r, hand_x);
        }
        let ink = if locked(o) { [80.0; 3] } else { BROWN };
        let size = (h * 0.6).min(18.0);
        let vw = 130.0f32.min(w * 0.36);
        let vx = x + w - vw - 4.0;
        d.text(label(o), x + 14.0, y + (h - size) / 2.0, size, ink);
        d.hit([x, y, vx - x, h], pointer::Act::Row(i, pointer::OK));
        d.box3([vx, y + 3.0, vw, h - 6.0], 8.0, [None, None, Some(if on { WHITE } else { [123.0, 115.0, 118.0] })], 128.0);
        let value = m.settings.value(o);
        d.label(&value, vx + vw / 2.0, y + (h - size) / 2.0, size, vw - 40.0, if value == "Off" { MUTED } else { ink });
        if let Volume(s) = o {
            let sw = 110.0f32.min(w * 0.22);
            let bar = [vx - sw - 12.0, y + h / 2.0 - 3.0, sw, 6.0];
            d.box3(bar, 3.0, [None, None, Some([118.0, 105.0, 110.0])], 128.0);
            d.box3([bar[0], bar[1], (sw * m.settings.2 .0[s] as f32 / 10.0).max(6.0), 6.0], 3.0, [None, None, Some([109.0, 58.0, 72.0])], 128.0);
        }
        if locked(o) {
            continue;
        }
        d.text("<", vx + 8.0, y + (h - size) / 2.0, size, BROWN);
        d.text(">", vx + vw - 8.0 - d.width(">", size), y + (h - size) / 2.0, size, BROWN);
        d.hit([vx, y, vw / 2.0, h], pointer::Act::Row(i, Press { left: true, ..default() }));
        d.hit([vx + vw / 2.0, y, vw / 2.0, h], pointer::Act::Row(i, Press { right: true, ..default() }));
    }
    d.text("Changes apply right away", list[0] + 16.0, list[1] + list[3] - 22.0, 12.0, MUTED);
    if let Some(&o) = opts.get(m.sel) {
        let preset = matches!(o, Scale | Shadows | Msaa | Fps | Vsync | Original);
        let note = if preset {
            "Set by Preset. Changing it switches Preset to Custom."
        } else if o == MenuSize {
            "Auto is 100%, 80% on screens taller than 1440 px. The match HUD keeps its size."
        } else {
            ""
        };
        // B324: the WebGL2 build says what it reaches here under the frame rate's and the renderer's rows
        #[cfg(target_arch = "wasm32")]
        let reach = matches!(o, Renderer | ShowFps).then(crate::web::webgl2_reach).flatten().map(|(fps, play, hz)| webgl2_reach(fps, play, hz));
        #[cfg(not(target_arch = "wasm32"))]
        let reach: Option<String> = None;
        let note = reach.as_deref().unwrap_or(note);
        let now = m.settings.value(o);
        side_panel(d, side, label(o), about(o), "Choices", &choices(o), Some(&now), note);
    }
}

/// B324: what the WebGL2 build reaches here, in words: `fps`, the best second's frame rate (`play`: in a match,
/// else in the menus), against the display's `refresh` rate (Hz, when the page could measure it). Short of the
/// display's rate by more than 10 % (a second's count wobbles by a frame, and the odd hitch), it's capped: say so and
/// where the full rate is to be had.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn webgl2_reach(fps: f64, play: bool, refresh: Option<f64>) -> String {
    let (fps, place) = (fps.round(), if play { "in a match" } else { "in the menus" });
    match refresh.map(f64::round) {
        Some(hz) if fps < hz * 0.9 => format!(
            "WebGL2 reaches up to {fps} fps {place} on this device, below the display's {hz} Hz. WebGPU (where the browser has it) or the desktop app (download the desktop build) gets the full refresh rate."
        ),
        Some(hz) => format!("WebGL2 reaches the display's full {hz} Hz {place} on this device."),
        None => format!("WebGL2 reaches up to {fps} fps {place} on this device."),
    }
}

/// The Controls page: each action with its keys and its pad buttons in two columns (✕ rebinds: the next key or
/// button pressed replaces that column's); the chosen one's bindings in the side panel.
pub(super) fn controls_layout(d: &mut Draw, m: &Menu, bind: &controls::Bindings, hand_x: f32) {
    let names: Vec<String> = controls::action_names().map(|n| n[..1].to_uppercase() + &n[1..].replace('_', " ")).collect();
    let (list, side) = chrome(d, m, names.len() + 1);
    let body = controls_body(list);
    let waiting = m.rebinding.is_some();
    let col = |r: [f32; 4], c: usize| {
        let vw = r[2] * 0.3;
        [r[0] + r[2] - 4.0 - (2 - c) as f32 * vw - (1 - c) as f32 * 6.0, vw]
    };
    let [_, _, _, h0] = row(body, names.len() + 1, 0);
    for (c, head) in ["Keyboard", "Controller"].into_iter().enumerate() {
        let [cx, vw] = col(row(body, names.len() + 1, 0), c);
        d.centred(head, cx + vw / 2.0, list[1] + 8.0, (h0 * 0.45).min(13.0), MUTED);
    }
    for (i, n) in names.iter().enumerate() {
        let r @ [x, y, _, h] = row(body, names.len() + 1, i);
        let on = i == m.sel;
        if on {
            chosen(d, r, hand_x);
        }
        let size = (h * 0.6).min(18.0);
        d.text(n, x + 14.0, y + (h - size) / 2.0, size, BROWN);
        let (pads, keys): (Vec<String>, Vec<String>) = bind.names(i).into_iter().partition(|n| n.starts_with("Pad"));
        for (c, v) in [keys, pads].into_iter().enumerate() {
            let [cx, vw] = col(r, c);
            let s = if on && waiting { "press...".to_string() } else if v.is_empty() { "-".to_string() } else { v.iter().map(|n| shown(n)).collect::<Vec<_>>().join("  ") };
            d.box3([cx, y + 3.0, vw, h - 6.0], 8.0, [None, None, Some(if on { WHITE } else { [123.0, 115.0, 118.0] })], 128.0);
            d.label(&s, cx + vw / 2.0, y + (h - size * 0.85) / 2.0, size * 0.85, vw - 16.0, if on && waiting || v.is_empty() { MUTED } else { BROWN });
        }
        d.hit(r, pointer::Act::Row(i, pointer::OK));
    }
    {
        // the last row: Reset to defaults
        let i = names.len();
        let r @ [x, y, _, h] = row(body, names.len() + 1, i);
        if i == m.sel {
            chosen(d, r, hand_x);
        }
        d.text("Reset to defaults", x + 14.0, y + (h - (h * 0.6).min(18.0)) / 2.0, (h * 0.6).min(18.0), BROWN);
        d.hit(r, pointer::Act::Row(i, pointer::OK));
    }
    d.text("Saved to controls.txt", list[0] + 16.0, list[1] + list[3] - 22.0, 12.0, MUTED);
    if let Some(n) = names.get(m.sel) {
        let about = if waiting { "Press the new key or button for it (Escape cancels). A key replaces its keys, a button its buttons." } else { "Choose it to rebind it: the next key or button you press." };
        let chips: Vec<String> = bind.names(m.sel).iter().map(|n| shown(n).to_string()).collect();
        side_panel(d, side, n, about, "Bound to", &chips, None, "");
    } else {
        side_panel(d, side, "Reset to defaults", "Puts every key and button back as it was first.", "", &[], None, "");
    }
}

/// The Controls page's rows' box: the list less the column headings on top.
pub(super) fn controls_body(list: [f32; 4]) -> [f32; 4] {
    [list[0], list[1] + 18.0, list[2], list[3] - 18.0]
}

/// A binding's name as the page shows it: `KeyW` W, `Digit1` 1, `PadDPadUp` DPadUp (controls.txt keeps the full ones).
fn shown(n: &str) -> &str {
    ["Pad", "Key", "Digit"].iter().find_map(|p| n.strip_prefix(p).filter(|r| !r.is_empty())).unwrap_or(n)
}

#[cfg(test)]
mod tests {
    use super::super::{Dev, Graphics, Volumes};
    use super::*;

    #[test]
    fn webgl2_reach_words() {
        let capped = webgl2_reach(59.6, true, Some(120.2));
        assert!(capped.starts_with("WebGL2 reaches up to 60 fps in a match on this device, below the display's 120 Hz.") && capped.contains("desktop build"), "{capped}");
        assert_eq!(webgl2_reach(116.0, false, Some(120.0)), "WebGL2 reaches the display's full 120 Hz in the menus on this device.");
        assert_eq!(webgl2_reach(60.0, true, Some(60.0)), "WebGL2 reaches the display's full 60 Hz in a match on this device.");
        assert_eq!(webgl2_reach(60.0, true, None), "WebGL2 reaches up to 60 fps in a match on this device.");
    }

    fn press(f: impl FnOnce(&mut Press) -> &mut bool) -> Press {
        let mut p = Press::default();
        *f(&mut p) = true;
        p
    }

    /// Every tab: ✕ on the main menu's Settings opens the first, R1 walks them in order and wraps, L1 back; ↓ walks each
    /// page's rows and wraps (changing nothing); ◀ ▶ on each option change it and save (◀ undoing ▶ on a cycle), each
    /// value it takes one of its side panel's choices; ○ lands on the main menu's Settings row.
    #[test]
    fn every_page() {
        let mut m = Menu::default();
        m.sel = m.main_at(super::super::home::Row::Settings);
        m.step(Dev::Keys, press(|k| &mut k.ok));
        for &(_, s) in tabs_shown() {
            assert_eq!((m.screen, m.sel), (s, 0));
            let n = if s == Screen::Controls { controls::action_names().count() + 1 } else { page(s).len() };
            for want in (1..n).chain([0]) {
                m.step(Dev::Keys, press(|k| &mut k.down));
                assert_eq!(m.sel, want);
            }
            assert_eq!(m.settings, Settings::default());
            for (i, &o) in page(s).iter().enumerate() {
                m.sel = i;
                let was = m.settings;
                let fx = m.step(Dev::Keys, press(|k| &mut k.right));
                if matches!(o, Volume(_)) {
                    m.step(Dev::Keys, press(|k| &mut k.left));
                    let fx = m.step(Dev::Keys, press(|k| &mut k.left));
                    assert!(fx.contains(&Effect::SaveSettings) && m.settings != was, "{o:?}");
                } else {
                    assert!(fx.contains(&Effect::SaveSettings) && m.settings != was, "{o:?}");
                    m.step(Dev::Keys, press(|k| &mut k.left));
                    assert_eq!(m.settings, was, "{o:?}");
                }
                for _ in 0..12 {
                    assert!(choices(o).contains(&m.settings.value(o)), "{o:?} {}", m.settings.value(o));
                    m.settings.step(o, -1);
                }
                m.settings = was;
            }
            m.sel = 0;
            m.step(Dev::Keys, press(|k| &mut k.next));
        }
        assert_eq!(m.screen, TABS[0].1);
        m.step(Dev::Keys, press(|k| &mut k.prev));
        assert_eq!(m.screen, tabs_shown().last().unwrap().1);
        m.step(Dev::Keys, press(|k| &mut k.back));
        assert_eq!((m.screen, m.sel), (Screen::Main, m.main_at(super::super::home::Row::Settings)));
    }

    /// B294: at any aspect every page's rows sit in its list over the footnote, the side panel has room, and the
    /// busiest page draws within the pause overlay's quad pool.
    #[test]
    fn pages_fit() {
        let t = super::super::pointer::tests::text_with_mods(0);
        for a in [4.0, 21.0 / 9.0, 16.0 / 9.0, 4.0 / 3.0, 5.0 / 4.0, 9.0 / 16.0, 0.4] {
            let grid = Grid::fit(a);
            for (_, s) in TABS {
                let n = if s == Screen::Controls { controls::action_names().count() + 1 } else { page(s).len() };
                let (list, side) = boxes(&grid, n);
                assert!(side[2] >= 150.0 && side[3] >= 150.0 && list[0] + list[2] <= side[0] + side[2], "{a}: {list:?} {side:?}");
                let r = row(list, n, n - 1);
                assert!(r[1] + r[3] <= list[1] + list[3] - 24.0 && r[3] >= 20.0, "{a} {s:?}");
                for sel in 0..n {
                    let m = Menu { screen: s, sel, grid, ..default() };
                    let q = super::super::drawn(&m, &t, &controls::Bindings::default(), &[], 0.0).out.len();
                    assert!(q < super::super::pause::POOL_HERE, "{a} {s:?} {sel}: {q} quads");
                }
            }
        }
    }

    /// Menu Size: at 60% on a 16:9 view the 640×448 screen is 60% of its 100% size, centred, its own shape; a tap
    /// lands on the scaled rect, not where it was at 100%; Auto is 80% past 1440 px tall; it round-trips settings.txt.
    #[test]
    fn menu_size_scales_centred() {
        use super::super::super::widescreen::{menu_shares, place, vplace_in};
        let (s, v) = menu_shares(16.0 / 9.0, 0.6);
        let (left, width) = place(0.0, 640.0, s, super::super::Anchor::Centre);
        let (top, height) = vplace_in(0.0, 448.0, v, super::super::Anchor::Centre);
        let close = |a: f32, b: f32| (a - b).abs() < 1e-4;
        assert!(close(width, 45.0) && close(left, 27.5) && close(height, 60.0) && close(top, 20.0), "{left} {width} {top} {height}");
        // same shape: 45% of 16 wide by 60% of 9 high is 4:3
        assert!(close(width * 16.0 / (height * 9.0), 4.0 / 3.0));
        let hits = [pointer::Hit { rect: [0.0, 0.0, 64.0, 44.8], anchor: default(), act: pointer::Act::Tab(0) }];
        let at = |x, y, k| pointer::hit_at(&hits, bevy::math::Vec2::new(x, y), 16.0 / 9.0, k, 0.0);
        assert_eq!(at(0.3, 0.22, 0.6), Some(pointer::Act::Tab(0)));
        assert_eq!(at(0.14, 0.02, 0.6), None);
        assert_eq!(at(0.14, 0.02, 1.0), Some(pointer::Act::Tab(0)));
        let mut set = Settings::default();
        assert_eq!((set.menu_scale(1440), set.menu_scale(2160), set.value(MenuSize).as_str()), (1.0, 0.8, "Auto"));
        set.step(MenuSize, 1);
        assert_eq!((set.menu_scale(2160), set.value(MenuSize).as_str()), (0.5, "50%"));
        set.step(MenuSize, -1);
        set.step(MenuSize, -1);
        assert_eq!(set.value(MenuSize), "150%");
        assert!(set.text().contains("menu_size = 150\n") && Settings::parse(&set.text()) == set);
    }

    /// B316: every row's choices are its own values — stepping one round through any row (the web build's Renderer
    /// too, which no native page shows) lands on each of its choices once and on nothing else.
    #[test]
    fn choices_are_each_rows_own_values() {
        let all = [DISPLAY, &[Display, Renderer], GRAPHICS, SOUND, MODS, MOBILE].concat();
        for o in all.into_iter().filter(|o| !matches!(o, Volume(_) | Preset)) {
            let mut s = Settings::default();
            let want = choices(o);
            let seen: Vec<_> = (0..want.len()).map(|_| (s.value(o), s.step(o, 1)).0).collect();
            assert_eq!((s.value(o), seen.len()), (seen[0].clone(), want.len()), "{o:?}");
            assert!(want.iter().all(|c| seen.contains(c)), "{o:?}: {seen:?} vs {want:?}");
        }
        assert_eq!(choices(Renderer), ["Auto", "WebGPU", "WebGL2"]);
        let mut s = Settings::default();
        s.step(Renderer, -1);
        assert_eq!(s.value(Renderer), "WebGL2");
    }

    /// B235: the sliders clamp to 0..=10 and the volumes round-trip settings.txt.
    #[test]
    fn sound_sliders() {
        let mut m = Menu { screen: Screen::Sound, sel: 1, ..default() };
        m.step(Dev::Keys, press(|k| &mut k.right));
        assert_eq!(m.settings.2, Volumes([10, 10, 10, 10]));
        (0..12).for_each(|_| _ = m.step(Dev::Keys, press(|k| &mut k.left)));
        assert_eq!(m.settings.2, Volumes([10, 0, 10, 10]));
        assert!(m.step(Dev::Keys, press(|k| &mut k.ok)).is_empty());
        assert_eq!(Settings::parse(&m.settings.text()), m.settings);
    }

    /// The preset steps through the presets; changing what it sets makes it Custom; Display and Show FPS survive a
    /// preset change; Upscaled Textures takes Upscaled Downloads with it.
    #[test]
    fn presets_and_switches() {
        let d = Graphics::default();
        let mut s = Settings::default();
        s.step(Preset, 1);
        assert_eq!(s.1, Graphics::STEAM_DECK);
        s.step(Preset, -1);
        assert_eq!(s.1, d);
        s.step(Scale, -1);
        assert_eq!((s.1.scale, s.value(Preset).as_str()), (85, "Custom"));
        s.step(Display, 1);
        s.step(ShowFps, 1);
        s.step(Preset, 1);
        assert_eq!((s.1.windowed, s.1.show_fps, s.value(Preset).as_str(), s.value(Display).as_str()), (true, 1, "Default", "Window"));
        assert!(Settings::parse(&s.text()).1.show_fps == 1 && s.flags().contains(&"--show-fps".into()));
        s.step(Original, 1);
        assert!(s.1.original && Settings::parse(&s.text()) == s && s.flags().contains(&"--original".into()));
        // B301: Key Hints, on by default, saved
        assert_eq!(s.value(KeyHints), "On");
        s.step(KeyHints, 1);
        assert!(!s.0[KEY_HINTS] && s.text().contains("key_hints = off") && Settings::parse(&s.text()) == s);
        s.step(Upscaled, 1);
        assert!(!s.0[0] && !s.0[UPSCALED_DOWNLOADS]);
        s.step(Downloads, 1);
        assert!(!s.0[0] && s.0[UPSCALED_DOWNLOADS]);
        s.step(Aspect, -1);
        assert_eq!(s.value(Aspect), ASPECTS[ASPECTS.len() - 1]);
        // B339: Downloads at Once steps Auto, 1..=8 and wraps; its side panel lists the same
        assert_eq!((s.value(AtOnce), choices(AtOnce).len()), ("Auto".into(), 9));
        s.step(AtOnce, -1);
        assert_eq!((s.value(AtOnce).as_str(), s.downloads_at_once()), ("8", 8));
        s.step(AtOnce, 1);
        s.step(AtOnce, 1);
        assert_eq!((s.value(AtOnce).as_str(), s.downloads_at_once(), Settings::parse(&s.text()) == s), ("1", 1, true));
    }
}
