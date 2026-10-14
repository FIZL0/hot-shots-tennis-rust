//! B290: the Settings screen's pages. Settings is a list of pages (Display, Graphics, Sound, Controls, Online & Mods);
//! every page but Controls is a column of options, each a row with its value (or volume slider) between ◀ ▶ arrows,
//! and Back.

use super::super::widescreen::ASPECTS;
use super::{Draw, Effect, GOLD, KEY_HINTS, Menu, Press, Screen, Settings, Tex, UPSCALED_DOWNLOADS, WHITE, pointer};
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
}
use Opt::*;

/// The Settings screen's rows (Back after them) and the pages they open.
pub(super) const HUB: [(&str, Screen); 5] =
    [("Display", Screen::Display), ("Graphics", Screen::Graphics), ("Sound", Screen::Sound), ("Controls", Screen::Controls), ("Online & Mods", Screen::ModSettings)];
/// The Settings screen's row for `page`, where Back from it lands.
pub(super) fn hub_row(page: Screen) -> usize {
    HUB.iter().position(|(_, s)| *s == page).unwrap_or(0)
}

/// The browser build stays in its page (`graphics::KEEP_WINDOWED`), so it has no Display mode row.
const DISPLAY: &[Opt] = if cfg!(target_arch = "wasm32") { &[Aspect, ShowFps, KeyHints] } else { &[Display, Aspect, ShowFps, KeyHints] };
/// What a preset sets, then the textures.
const GRAPHICS: &[Opt] = &[Preset, Scale, Shadows, Msaa, Fps, Vsync, Original, Upscaled];
const SOUND: &[Opt] = &[Volume(0), Volume(1), Volume(2), Volume(3), MenuMusic, MatchMusic];
const MODS: &[Opt] = &[Downloads, OnlineMods];
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
            Fps if g.fps == 0 => "Uncapped".into(),
            Fps => format!("{} fps", g.fps),
            Vsync => on_off(g.vsync),
            Original => on_off(g.original),
            Upscaled => on_off(self.0[0]),
            Volume(i) => self.2 .0[i].to_string(),
            MenuMusic => on_off(self.0[1]),
            MatchMusic => on_off(self.0[2]),
            Downloads => on_off(self.0[UPSCALED_DOWNLOADS]),
            OnlineMods => ASK_VALUES.iter().find(|(a, _)| *a == self.4).unwrap().1.into(),
        }
    }
}

impl Menu {
    /// The Settings screen: ✕ opens a page, Back (or ○) returns to the main menu.
    pub(super) fn step_hub(&mut self, k: Press, out: &mut Vec<Effect>) {
        if k.ok && let Some((_, s)) = HUB.get(self.sel) {
            self.go(*s, out);
        } else if k.ok || k.back {
            self.sel = 2;
            self.screen = Screen::Main;
            out.push(Effect::Sound(1));
        }
    }

    /// An options page: ◀ ▶ (and ✕, but on a slider) change the row's option and save; Back returns to Settings.
    pub(super) fn step_page(&mut self, k: Press, out: &mut Vec<Effect>) {
        let opts = page(self.screen);
        if let Some(&o) = opts.get(self.sel) {
            if !locked(o) && (k.left || k.right || (k.ok && ok_changes(o))) {
                self.settings.step(o, if k.left { -1 } else { 1 });
                out.extend([Effect::Sound(0), Effect::SaveSettings]);
                return;
            }
        } else if k.ok {
            return self.back_to_hub(out);
        }
        if k.back {
            self.back_to_hub(out);
        }
    }

    /// Back from a page to its row on the Settings screen.
    pub(super) fn back_to_hub(&mut self, out: &mut Vec<Effect>) {
        self.sel = hub_row(self.screen);
        self.screen = Screen::Settings;
        out.push(Effect::Sound(1));
    }
}

/// The rows' first y and pitch (640×448): Graphics' nine rows, Back included, end above the description bar (y 404).
pub(super) const TOP: f32 = 46.0;
pub(super) const PITCH: f32 = 39.5;

/// The Settings screen.
pub(super) fn hub_layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    d.frame(1, 6);
    let labels: Vec<String> = HUB.iter().map(|(l, _)| l.to_string()).chain(["Back".into()]).collect();
    d.rows_at(&labels, m.sel, 170.0, TOP, 300.0, hand_x, PITCH);
    d.info(match m.sel {
        0 => "Fullscreen or a window, the aspect ratio and the frame-rate counter.",
        1 => "Quality preset, resolution, shadows, anti-aliasing, frame limit, vsync and textures.",
        2 => "The volumes, and music in the menus and in a match.",
        3 => "Change the keys and buttons (controls.txt).",
        4 => "What the mod store and online lobbies download.",
        _ => "Back to the main menu.",
    });
}

/// An options page: the rows on the left, each value between its arrows on the right (a tap on the value's left half
/// steps back, on its right half on, B221), a slider as a gold bar.
pub(super) fn page_layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    d.frame(1, 6);
    let opts = page(m.screen);
    let labels: Vec<String> = opts.iter().map(|o| label(*o).to_string()).chain(["Back".into()]).collect();
    d.rows_locked(&labels, m.sel, 120.0, TOP, 300.0, hand_x, PITCH, |i| opts.get(i).is_some_and(|o| locked(*o)));
    for (i, &o) in opts.iter().enumerate() {
        let y = TOP + PITCH * i as f32;
        let rgb = if locked(o) { [50.0; 3] } else if i == m.sel { WHITE } else { [40.0; 3] };
        if let Volume(s) = o {
            d.q(Tex::Solid, [0.0; 4], [444.0, y + 14.0, 160.0, 10.0], [15.0; 3], 128.0);
            d.q(Tex::Solid, [0.0; 4], [444.0, y + 14.0, 16.0 * m.settings.2 .0[s] as f32, 10.0], if i == m.sel { GOLD } else { [100.0, 90.0, 40.0] }, 128.0);
        } else {
            d.centred(&m.settings.value(o), 527.0, y + 7.0, 24.0, rgb);
        }
        if locked(o) {
            continue;
        }
        d.text("<", 424.0, y + 7.0, 24.0, rgb);
        d.text(">", 614.0, y + 7.0, 24.0, rgb);
        d.hit([420.0, y, 100.0, PITCH], pointer::Act::Row(i, Press { left: true, ..default() }));
        d.hit([520.0, y, 115.0, PITCH], pointer::Act::Row(i, Press { right: true, ..default() }));
    }
    d.info(opts.get(m.sel).map_or("Back to the settings.", |o| about(*o)));
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

    /// Every page: ↓ walks each row to Back and wraps (changing nothing), every row ends above the description bar,
    /// ◀ ▶ on each option change it and save (◀ undoing ▶ on a cycle), Back lands on the page's Settings row.
    #[test]
    fn every_page() {
        for (row, (_, s)) in HUB.iter().enumerate() {
            let mut m = Menu { screen: Screen::Settings, sel: row, ..default() };
            m.step(Dev::Keys, press(|k| &mut k.ok));
            assert_eq!((m.screen, m.sel), (*s, 0));
            if *s == Screen::Controls {
                continue;
            }
            let n = page(*s).len() + 1;
            assert!(TOP + PITCH * (n - 1) as f32 + 38.0 <= 404.0, "{s:?}");
            for want in (1..n).chain([0]) {
                m.step(Dev::Keys, press(|k| &mut k.down));
                assert_eq!(m.sel, want);
            }
            assert_eq!(m.settings, Settings::default());
            for (i, &o) in page(*s).iter().enumerate() {
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
                m.settings = was;
            }
            m.step(Dev::Keys, press(|k| &mut k.back));
            assert_eq!((m.screen, m.sel), (Screen::Settings, row));
        }
        // Back on Settings: the main menu, on its Settings row
        let mut m = Menu { screen: Screen::Settings, sel: HUB.len(), ..default() };
        m.step(Dev::Keys, press(|k| &mut k.ok));
        assert_eq!((m.screen, m.sel), (Screen::Main, 2));
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
