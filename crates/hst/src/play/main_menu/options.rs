//! B290: the Settings screen's pages (Display, Graphics, Sound, Controls, Online & Mods); every page but Controls is a
//! column of options, each a row with its value (or volume slider) between ◀ ▶ arrows.
//! B294 (user's mockup, 2026-10-14): the pages are tabs across the title band (L1 / R1), the rows compact in a list
//! panel, a side panel with the chosen option's name, description and choices; no description bar (the legend
//! strip's keys only); laid out on the whole view at any aspect (the select's [`Grid`] says how far it reaches).

use super::super::widescreen::ASPECTS;
use super::{BROWN, CHAT, CELL_FILL, CELL_ON, CREAM, Draw, Effect, Grid, INK, KEY_HINTS, Menu, Press, RENDERERS, Screen, Settings, Tex, UPSCALED_DOWNLOADS, WHITE, controls, pointer};
use crate::mods_online::Ask;
use crate::graphics::{FPS, Graphics, MSAA, SCALES, SHADOWS};
use bevy::prelude::default;

/// One option row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Opt {
    Display,
    Aspect,
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
    /// `Volumes`' slider `i`, 0..=10.
    Volume(usize),
    MenuMusic,
    MatchMusic,
    Downloads,
    /// B289: what to do with an online lobby's mods.
    OnlineMods,
    /// M20: online chat shown.
    Chat,
    /// B273: the browser build's renderer.
    Renderer,
}
use Opt::*;

/// The Settings tabs, in order, and the pages they open.
pub(super) const TABS: [(&str, Screen); 5] =
    [("Display", Screen::Display), ("Graphics", Screen::Graphics), ("Sound", Screen::Sound), ("Controls", Screen::Controls), ("Online & Mods", Screen::ModSettings)];
/// `page`'s tab.
pub(super) fn tab(page: Screen) -> usize {
    TABS.iter().position(|(_, s)| *s == page).unwrap_or(0)
}

/// The browser build stays in its page (`graphics::KEEP_WINDOWED`), so it has no Display mode row; it has a
/// Renderer instead (B273).
const DISPLAY: &[Opt] = if cfg!(target_arch = "wasm32") { &[Renderer, Aspect, ShowFps, KeyHints] } else { &[Display, Aspect, ShowFps, KeyHints] };
/// What a preset sets, then the textures.
const GRAPHICS: &[Opt] = &[Preset, Scale, Shadows, Msaa, Fps, Vsync, Original, Upscaled];
const SOUND: &[Opt] = &[Volume(0), Volume(1), Volume(2), Volume(3), MenuMusic, MatchMusic];
const MODS: &[Opt] = &[Downloads, OnlineMods, Chat];
/// B289: Online Mod Downloads' values.
const ASK_VALUES: [(Ask, &str); 3] = [(Ask::Each, "Ask"), (Ask::Always, "Always"), (Ask::Never, "Never")];

/// Page `s`'s options (none on a screen that isn't an options page).
pub(super) fn page(s: Screen) -> &'static [Opt] {
    match s {
        Screen::Display => DISPLAY,
        Screen::Graphics => GRAPHICS,
        Screen::Sound => SOUND,
        Screen::ModSettings => MODS,
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
        Volume(i) => ["Master", "Music", "Sound Effects", "Voices"][i],
        MenuMusic => "Menu Music",
        MatchMusic => "Match Music",
        Downloads => "Upscaled Downloads",
        OnlineMods => "Online Mod Downloads",
        Chat => "Chat",
        Renderer => "Renderer",
    }
}

fn about(o: Opt) -> &'static str {
    match o {
        Display => "Borderless fullscreen on the current monitor, or a window.",
        Aspect => "Auto fills the window; a fixed ratio shows the whole picture at that shape, black bars around it.",
        ShowFps => "Show the frame rate in the top-right corner, in the menus and in a match.",
        KeyHints => "Show the keyboard's keys at the bottom during a match while you play on the keyboard.",
        Preset => "Default draws at full quality; Steam Deck and Low trade detail for frame rate.",
        Scale => "Draw the court at this share of the window's resolution, scaled up to fill it (the HUD stays sharp).",
        Shadows => "Off, or the shadow map's detail (Low: smaller and fewer cascades).",
        Msaa => "Smooth the court's jagged edges (MSAA): off, 2x or 4x samples per pixel.",
        Fps => "The most frames drawn a second (the game itself always runs at 60 Hz).",
        Vsync => "Wait for the display between frames: no tearing, at most its refresh rate.",
        Original => "Draw at the PS2's 640x448, 4:3, with the disc's textures, scaled up unfiltered.",
        Upscaled | Downloads if locked(o) => "Upscaled textures aren't available in the browser.",
        Upscaled => "Use the upscaled textures in mods/texture-replacements.",
        Volume(0) => "The volume of everything.",
        Volume(1) => "The menu and match music and the jingles.",
        Volume(2) => "The court, the crowd, the weather and the menu sounds.",
        Volume(_) => "The players' and the umpire's voices.",
        MenuMusic => "Play music in the menus.",
        MatchMusic => "Play each court's music during a match.",
        Downloads => "Download mods' upscaled textures from the mod store (most of a mod's size).",
        OnlineMods => "When an online lobby has a mod you don't have: ask each time, always download it, or never (a stand-in plays it).",
        Chat => "Show the online lobby's and match's text chat (Enter, or R2/L2 on a pad), or hide it.",
        Renderer => "Auto: WebGPU when the browser has it, else WebGL2. The page reloads into it a moment after a change.",
    }
}

fn fps_name(fps: u32) -> String {
    if fps == 0 { "Uncapped".into() } else { format!("{fps} fps") }
}

/// B294: every value `o` can take, in order, as [`Settings::value`] words it (the side panel's choices).
fn choices(o: Opt) -> Vec<String> {
    let words = |v: &[&str]| v.iter().map(|w| w.to_string()).collect();
    match o {
        _ if locked(o) => words(&["Off"]),
        Display => words(&["Fullscreen", "Window"]),
        Aspect => words(&ASPECTS),
        Preset => Graphics::PRESETS.iter().map(|(n, _)| n.to_string()).chain(["Custom".into()]).collect(),
        Scale => SCALES.iter().map(|s| format!("{s} %")).collect(),
        Shadows | Msaa => words(if o == Shadows { &["Off", "Low", "High"] } else { &["Off", "2x", "4x"] }),
        Fps => FPS.map(fps_name).to_vec(),
        Volume(_) => (0..=10).map(|v| v.to_string()).collect(),
        OnlineMods => ASK_VALUES.iter().map(|(_, n)| n.to_string()).collect(),
        _ => words(&["Off", "On"]),
    }
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
            ShowFps => g.show_fps = !g.show_fps,
            KeyHints => self.0[KEY_HINTS] = !self.0[KEY_HINTS],
            Preset => {
                let names: Vec<_> = Graphics::PRESETS.iter().map(|(n, _)| *n).collect();
                let n = cycle(&names, g.preset(), d);
                *g = Graphics { windowed: g.windowed, show_fps: g.show_fps, ..Graphics::PRESETS.iter().find(|(m, _)| *m == n).unwrap().1 };
            }
            Scale => g.scale = cycle(&SCALES, g.scale, d),
            Shadows => g.shadows = cycle(&SHADOWS, g.shadows, d),
            Msaa => g.msaa = cycle(&MSAA, g.msaa, d),
            Fps => g.fps = cycle(&FPS, g.fps, d),
            Vsync => g.vsync = !g.vsync,
            Original => g.original = !g.original,
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
            OnlineMods => self.4 = cycle(&ASK_VALUES.map(|(a, _)| a), self.4, d),
            Chat => {
                self.0[CHAT] = !self.0[CHAT];
                crate::chat::HIDDEN.store(!self.0[CHAT], std::sync::atomic::Ordering::Relaxed);
            }
            Renderer => self.5 = (self.5 as isize + d).rem_euclid(RENDERERS.len() as isize) as u8,
        }
    }

    /// The value shown beside `o` (a slider's is its number).
    pub(super) fn value(&self, o: Opt) -> String {
        let g = &self.1;
        match o {
            _ if locked(o) => "Off".into(),
            Display => if g.windowed { "Window" } else { "Fullscreen" }.into(),
            Aspect => ASPECTS[self.3 as usize].into(),
            ShowFps => on_off(g.show_fps),
            KeyHints => on_off(self.0[KEY_HINTS]),
            Preset => g.preset().into(),
            Scale => format!("{} %", g.scale),
            Shadows => ["Off", "Low", "High"][g.shadows as usize].into(),
            Msaa => ["Off", "2x", "4x"][g.msaa.ilog2().min(2) as usize].into(),
            Fps => fps_name(g.fps),
            Vsync => on_off(g.vsync),
            Original => on_off(g.original),
            Upscaled => on_off(self.0[0]),
            Volume(i) => self.2 .0[i].to_string(),
            MenuMusic => on_off(self.0[1]),
            MatchMusic => on_off(self.0[2]),
            Downloads => on_off(self.0[UPSCALED_DOWNLOADS]),
            OnlineMods => ASK_VALUES.iter().find(|(a, _)| *a == self.4).unwrap().1.into(),
            Chat => on_off(self.0[CHAT]),
            Renderer => ["Auto", "WebGPU", "WebGL2"][self.5 as usize].into(),
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
            let n = TABS.len();
            self.go(TABS[(tab(self.screen) + if k.next { 1 } else { n - 1 }) % n].1, out);
        } else if k.back {
            self.sel = 2;
            self.screen = Screen::Main;
            out.push(Effect::Sound(1));
        }
    }
}

/// Words' purple-grey (the mockup's #7a4a5c): footnotes and headings.
const MUTED: [f32; 3] = [61.0, 37.0, 46.0];

/// B294: the Settings pages' list (of `n` rows) and side panel on the view `g` fits (the side panel under the list in a portrait
/// window), from under the tabs down to the legend strip.
pub(super) fn boxes(g: &Grid, n: usize) -> ([f32; 4], [f32; 4]) {
    let [l, y, w, _] = g.strip;
    let bottom = g.detail[1] + g.detail[3] + 32.0;
    if g.detail[1] > g.roster[1] {
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
    // right of the title's tab (`Draw::frame`), to the window's right edge (the strip spans the view less 12 a side)
    let x0 = d.width(super::TITLES[1].0, 30.0) + 88.0;
    let room = m.grid.strip[2] + 24.0 - x0 - 8.0;
    let ws: Vec<f32> = TABS.iter().map(|(l, _)| d.width(l, 15.0) + 22.0).collect();
    let k = (room / (ws.iter().sum::<f32>() + 6.0 * (TABS.len() - 1) as f32)).min(1.0);
    d.anchor = super::super::widescreen::Anchor::Span;
    let mut x = x0;
    for (i, ((words, s), w)) in TABS.iter().zip(&ws).enumerate() {
        let on = *s == m.screen;
        let b = [x, 3.0, w * k, 24.0];
        d.pill_at(b, [INK, if on { INK } else { [90.0; 3] }, if on { BROWN } else { CELL_FILL }], 128.0);
        d.label(words, x + w * k / 2.0, 7.5, 15.0, (w - 18.0) * k, if on { WHITE } else { BROWN });
        d.hit(b, pointer::Act::Tab(i));
        x += (w + 6.0) * k;
    }
    d.anchor = super::super::widescreen::Anchor::Centre;
}

/// `s` broken at spaces to lines `w` wide at `size` from (x, y), `gap` px apart; the y under its last line.
fn para(d: &mut Draw, s: &str, x: f32, y: f32, w: f32, size: f32, gap: f32, rgb: [f32; 3]) -> f32 {
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
fn chosen(d: &mut Draw, r: [f32; 4], hand_x: f32) {
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
        let note = if preset { "Set by Preset. Changing it switches Preset to Custom." } else { "" };
        let now = m.settings.value(o);
        side_panel(d, side, label(o), about(o), "Choices", &choices(o), Some(&now), note);
    }
}

/// The Controls page: each action and its keys and buttons (✕ rebinds: the next key or button pressed); the chosen
/// one's bindings in the side panel.
pub(super) fn controls_layout(d: &mut Draw, m: &Menu, bind: &controls::Bindings, hand_x: f32) {
    let names: Vec<String> = controls::action_names().map(|n| n[..1].to_uppercase() + &n[1..].replace('_', " ")).collect();
    let (list, side) = chrome(d, m, names.len());
    let waiting = m.rebinding.is_some();
    for (i, n) in names.iter().enumerate() {
        let r @ [x, y, w, h] = row(list, names.len(), i);
        let on = i == m.sel;
        if on {
            chosen(d, r, hand_x);
        }
        let size = (h * 0.6).min(18.0);
        d.text(n, x + 14.0, y + (h - size) / 2.0, size, BROWN);
        let keys = if on && waiting { "press a key or button...".to_string() } else { bind.names(i).join("  ") };
        let vw = w * 0.6;
        d.box3([x + w - vw - 4.0, y + 3.0, vw, h - 6.0], 8.0, [None, None, Some(if on { WHITE } else { [123.0, 115.0, 118.0] })], 128.0);
        d.label(&keys, x + w - vw / 2.0 - 4.0, y + (h - size * 0.85) / 2.0, size * 0.85, vw - 16.0, if on && waiting { MUTED } else { BROWN });
        d.hit(r, pointer::Act::Row(i, pointer::OK));
    }
    d.text("Saved to controls.txt", list[0] + 16.0, list[1] + list[3] - 22.0, 12.0, MUTED);
    if let Some(n) = names.get(m.sel) {
        let about = if waiting { "Press the new key or button for it (Escape cancels). A key replaces its keys, a button its buttons." } else { "Choose it to rebind it: the next key or button you press." };
        side_panel(d, side, n, about, "Bound to", &bind.names(m.sel), None, "");
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Dev, Graphics, Volumes};
    use super::*;

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
        let mut m = Menu { sel: 2, ..default() };
        m.step(Dev::Keys, press(|k| &mut k.ok));
        for (_, s) in TABS {
            assert_eq!((m.screen, m.sel), (s, 0));
            let n = if s == Screen::Controls { controls::action_names().count() } else { page(s).len() };
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
        assert_eq!(m.screen, TABS[4].1);
        m.step(Dev::Keys, press(|k| &mut k.back));
        assert_eq!((m.screen, m.sel), (Screen::Main, 2));
    }

    /// B294: at any aspect every page's rows sit in its list over the footnote, the side panel has room, and the
    /// busiest page draws within the pause overlay's quad pool.
    #[test]
    fn pages_fit() {
        let t = super::super::pointer::tests::text_with_mods(0);
        for a in [4.0, 21.0 / 9.0, 16.0 / 9.0, 4.0 / 3.0, 5.0 / 4.0, 9.0 / 16.0, 0.4] {
            let grid = Grid::fit(a);
            for (_, s) in TABS {
                let n = if s == Screen::Controls { controls::action_names().count() } else { page(s).len() };
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
        assert_eq!((s.1.windowed, s.1.show_fps, s.value(Preset).as_str(), s.value(Display).as_str()), (true, true, "Default", "Window"));
        assert!(Settings::parse(&s.text()).1.show_fps && s.flags().contains(&"--show-fps".into()));
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
    }
}
