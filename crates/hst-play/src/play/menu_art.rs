//! B136: the menus' remastered art (ours, by the user's request). A menu sheet named in [`SHEETS`] is rebuilt at load
//! time at [`K`]× its size, in the same layout, so every screen's pixel rects still pick the same pieces
//! (`textures::ours` scales them): each label listed for it is erased from the disc picture and set again as vector
//! text in M PLUS Rounded 1c (OFL, `crates/hst/assets/fonts/`), filled with the label's own colours (sampled per row from the
//! disc, outline colour and width too), so it stays sharp at any resolution; what's left (pills, boxes, icons) is
//! scaled up with its edges snapped sharp. Only the label rects and their words are ours in the source; the pixels come
//! from the user's disc at run time. `--original` (`OFF`) keeps the disc's sheets.
//!
//! `HST_MENU_ART=DIR` saves each rebuilt sheet there as PNG (for checking against the disc's).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use hst_data::tim2;
use std::sync::atomic::{AtomicBool, Ordering};
use tiny_skia::{Color as SkColor, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint, PathBuilder, Pixmap, Point, SpreadMode, Stroke, Transform};

/// Set by `--original`: the disc's own sheets.
pub static OFF: AtomicBool = AtomicBool::new(false);

/// The rebuilt sheets' scale over the disc's.
pub const K: u32 = 4;

/// A wall's scale over the disc's tile (the tiled quad divides its stretch by it): `K`, or 1 with `--original`.
pub fn wall_k() -> f32 {
    if OFF.load(Ordering::Relaxed) { 1.0 } else { K as f32 }
}

static HEAVY: &[u8] = include_bytes!("../../../hst/assets/fonts/MPLUSRounded1c-ExtraBold.ttf");
pub(crate) static BOLD: &[u8] = include_bytes!("../../../hst/assets/fonts/MPLUSRounded1c-Bold.ttf");

/// A label: its ink box on the disc sheet (x, y, w, h) and its words.
type Label<'a> = (u16, u16, u16, u16, &'a str);

struct Sheet {
    name: &'static str,
    labels: &'static [Label<'static>],
    /// A screen's title bar: these words where the disc's title stands on its stripes (box found at run time), with a
    /// drop shadow.
    title: Option<&'static str>,
    /// B141a: stylised words and digits of ours, each cell cleared and stamped in its [`Look`].
    stamps: &'static [Stamp],
    /// B141a: icons of ours drawn over the sheet (each clears its own cells).
    icons: Option<fn(&mut Pixmap)>,
}

const fn sheet(name: &'static str, labels: &'static [Label<'static>]) -> Sheet {
    Sheet { name, labels, title: None, stamps: &[], icons: None }
}

const fn title(name: &'static str, words: &'static str) -> Sheet {
    Sheet { name, labels: &[], title: Some(words), stamps: &[], icons: None }
}

const fn stamped(name: &'static str, labels: &'static [Label<'static>], stamps: &'static [Stamp]) -> Sheet {
    Sheet { name, labels, title: None, stamps, icons: None }
}

/// Rounded bars drawn again as shapes (x, y, w, h on the disc sheet): a 2-px dark rim, a 2-px white ring and a flat
/// fill, each colour taken from the disc bar, so their round ends stay smooth (the mode pills).
const PILLS: &[(&str, &[(u16, u16, u16, u16)])] = &[("characterselect2_00", &[(0, 0, 256, 32), (0, 32, 256, 32)])];

const SHEETS: &[Sheet] = &[
    sheet("option_00", OPTION_00),
    sheet("word", &[]),
    sheet("characterselect2_00", CS0),
    sheet("characterselect2_02", NAMES),
    sheet("characterselect2_04", ATTRS),
    sheet("keyassign_characterselect2", KEYS),
    sheet("confirmation_01", COURTS),
    stamped("confirmation_03", CONFIRM, &[((448, 268, 64, 41), "VS", VS)]),
    sheet("confirmation_04", RANKS),
    sheet("i_dialog_01", DIALOG),
    sheet("loading", &[(2, 152, 195, 32, "Now Loading...")]),
    sheet("inpane_pause02", PAUSED),
    sheet("inpane_team", TEAMS),
    sheet("result_status01", STATS),
    sheet("keyassign_inpane", RESULT_KEYS),
    title("title_mainmenu", "Main Menu"),
    title("title_option", "Options"),
    title("title_playerselect", "Game Selection"),
    title("title_characterselect2", "Character Selection"),
    title("title_confirmation_01", "Confirm Settings (Fun Time Tennis)"),
    stamped("result_gameset01", &[], &DIGITS_PAST),
    stamped("result_gameset02", &[], &DIGITS_NOW),
    stamped("result_gameset05", &[], BANNERS),
    stamped("inpane_pause00", &[], POINTS),
    stamped("inpane_pause01", &[], TIEBREAK),
    Sheet { name: "info", labels: &[], title: None, stamps: &[], icons: Some(info_icons) },
];


/// B141a: our look for a stylised word or digit (nothing sampled from the disc): a fill fading from `top` to `bottom`,
/// a dark `edge` outline `ew` disc px wide and a light `ring` `rw` px round that, slanted by `slant` (x per y).
#[derive(Clone, Copy)]
pub(crate) struct Look {
    pub top: [u8; 3],
    pub bottom: [u8; 3],
    pub edge: [u8; 3],
    pub ring: [u8; 3],
    pub ew: f32,
    pub rw: f32,
    pub slant: f32,
}

const fn look(top: [u8; 3], bottom: [u8; 3], edge: [u8; 3], ew: f32, slant: f32) -> Look {
    Look { top, bottom, edge, ring: [255; 3], ew, rw: ew * 0.75, slant }
}

/// A stamp: its cell on the disc sheet (x, y, w, h: the outer edge of its ring), its words and look.
type Stamp = ((u16, u16, u16, u16), &'static str, Look);

/// The score digits: a past set's games (green) and the set in play (orange), 32-px cells, "-" after the 9.
const DIGITS_PAST: [Stamp; 11] = digits(look([200, 250, 160], [50, 150, 50], [20, 70, 20], 2.0, 0.0));
const DIGITS_NOW: [Stamp; 11] = digits(look([255, 225, 160], [220, 100, 30], [95, 40, 10], 2.0, 0.0));

const fn digits(l: Look) -> [Stamp; 11] {
    const S: [&str; 11] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "-"];
    let mut out = [((0, 0, 0, 0), "", l); 11];
    let mut d = 0;
    while d < 11 {
        out[d] = ((d as u16 * 32 + 2, 2, if d == 10 { 20 } else { 28 }, 28), S[d], l);
        d += 1;
    }
    out
}

/// The result banners, one 80-px row each (`ceremony`'s rows).
const BANNERS: &[Stamp] = &[
    ((8, 12, 240, 54), "DOMINATION", look([255, 245, 140], [200, 150, 0], [80, 60, 0], 3.0, 0.0)),
    ((8, 92, 240, 54), "ANNIHILATION", look([225, 232, 236], [80, 95, 108], [28, 34, 40], 3.0, 0.0)),
    ((8, 172, 240, 54), "VICTORY", look([255, 210, 150], [225, 60, 30], [100, 20, 10], 3.0, 0.0)),
    ((8, 252, 240, 54), "DEFEAT", look([235, 205, 255], [125, 75, 180], [50, 20, 80], 3.0, 0.0)),
    ((8, 332, 240, 54), "NARROW WIN", look([255, 230, 195], [200, 125, 65], [90, 50, 20], 3.0, 0.0)),
    ((8, 412, 240, 54), "NARROW LOSS", look([195, 232, 255], [35, 125, 200], [10, 45, 90], 3.0, 0.0)),
];

/// The confirm screen's "VS".
const VS: Look = look([255, 225, 80], [230, 70, 20], [120, 30, 10], 2.5, 0.2);

/// The pause board's points (64×48 cells as `menu::POINT_U`/`POINT_V` place them): gold, the tiebreak's red.
const GOLD: Look = look([255, 240, 130], [230, 160, 20], [90, 50, 10], 2.5, 0.15);
const RED: Look = look([255, 160, 160], [190, 35, 50], [80, 10, 20], 2.5, 0.15);
const POINTS: &[Stamp] = &[
    ((4, 4, 56, 40), "0", GOLD),
    ((68, 4, 56, 40), "15", GOLD),
    ((4, 52, 56, 40), "30", GOLD),
    ((68, 52, 56, 40), "40", GOLD),
    ((4, 100, 56, 40), "D", GOLD),
    ((68, 100, 56, 40), "A", GOLD),
];
const TIEBREAK: &[Stamp] = &[
    ((4, 4, 56, 40), "0", RED),
    ((68, 4, 56, 40), "1", RED),
    ((4, 52, 56, 40), "2", RED),
    ((68, 52, 56, 40), "3", RED),
    ((4, 100, 56, 40), "4", RED),
    ((68, 100, 56, 40), "5", RED),
    ((4, 148, 56, 40), "6", RED),
    ((68, 148, 56, 40), "7", RED),
    ((4, 196, 56, 40), "D", RED),
    ((68, 196, 56, 40), "A", RED),
];

/// The mode bars, card labels, play styles and the stat names of the select (CS0).
const CS0: &[Label<'static>] = &[
    (88, 5, 80, 22, "Singles"),
    (84, 37, 89, 21, "Doubles"),
    (299, 5, 34, 13, "Serve"),
    (297, 21, 38, 13, "Stroke"),
    (298, 37, 36, 14, "Volley"),
    (297, 53, 39, 14, "Impact"),
    (290, 69, 53, 13, "Footwork"),
    (293, 85, 47, 13, "Stamina"),
    (190, 66, 28, 20, "1P"),
    (230, 66, 29, 20, "2P"),
    (189, 90, 30, 20, "3P"),
    (229, 90, 30, 20, "4P"),
    (145, 116, 43, 17, "COM"),
    (89, 140, 21, 7, "Both"),
    (89, 149, 26, 7, "hands"),
    (89, 164, 24, 9, "Right"),
    (89, 173, 31, 7, "hander"),
    (89, 188, 17, 7, "Left"),
    (89, 197, 31, 7, "hander"),
    (320, 115, 144, 21, "All-round Player"),
    (326, 139, 133, 21, "Baseline Player"),
    (347, 163, 91, 21, "Net Player"),
    (345, 187, 93, 23, "Big Server"),
    (4, 211, 104, 21, "Ready to go!"),
    (2, 236, 109, 18, "Not selectable"),
    (402, 273, 25, 16, "Flat"),
    (399, 297, 30, 16, "Slice"),
    (391, 321, 45, 17, "Topspin"),
    (394, 345, 40, 16, "Smash"),
    (393, 369, 42, 18, "Volley"),
    (275, 364, 74, 16, "All-round"),
    (279, 388, 66, 16, "Baseline"),
    (272, 412, 81, 18, "Net Player"),
    (272, 435, 81, 19, "Big Server"),
];

/// The characters' names on the select's cards.
const NAMES: &[Label<'static>] = &[
    (32, 2, 64, 21, "Ashley"),
    (41, 26, 48, 21, "Cody"),
    (47, 50, 34, 18, "Jun"),
    (41, 74, 48, 18, "Kaito"),
    (27, 98, 76, 18, "Momoko"),
    (54, 122, 20, 18, "JJ"),
    (40, 146, 49, 18, "Carol"),
    (23, 170, 84, 21, "Big Chief"),
    (28, 194, 75, 18, "Miranda"),
    (43, 218, 44, 18, "Kent"),
    (45, 242, 41, 18, "Lola"),
    (46, 266, 36, 18, "Will"),
    (36, 290, 57, 18, "Gloria"),
    (33, 314, 62, 18, "Suzuki"),
];

/// The attribute card: play styles, stat names and the A–F grades.
const ATTRS: &[Label<'static>] = &[
    (33, 81, 74, 16, "All-round"),
    (46, 96, 55, 15, "Player"),
    (37, 121, 66, 15, "Baseline"),
    (40, 135, 61, 16, "Player"),
    (28, 169, 85, 19, "Net Player"),
    (28, 209, 85, 19, "Big Server"),
    (21, 248, 46, 19, "Serve"),
    (18, 273, 52, 18, "Stroke"),
    (19, 297, 50, 20, "Volley"),
    (18, 321, 53, 20, "Impact"),
    (9, 345, 71, 18, "Footwork"),
    (235, 3, 17, 17, "A"),
    (237, 27, 15, 17, "B"),
    (236, 51, 16, 18, "C"),
    (236, 75, 16, 17, "D"),
    (237, 99, 14, 17, "E"),
    (237, 123, 14, 17, "F"),
];

/// The select's key hints.
const KEYS: &[Label<'static>] = &[(0, 5, 103, 15, "Show Attributes"), (0, 28, 130, 17, "Select Costume"), (1, 50, 154, 20, "Select Costume")];

/// Court names, surfaces and the umpires' names.
const COURTS: &[Label<'static>] = &[
    (0, 2, 214, 21, "North Academy Grounds"),
    (0, 26, 227, 18, "Aloha Beach Tennis Court"),
    (0, 50, 252, 18, "Western Prairie Tennis Court"),
    (0, 74, 221, 21, "Sakuragaoka Park Courts"),
    (0, 98, 204, 20, "Queen's Tennis Garden"),
    (0, 122, 183, 21, "Alpine Heights Court"),
    (0, 146, 229, 18, "Grand Palais Indoor Court"),
    (0, 170, 214, 18, "Wild Green Tennis Court"),
    (0, 194, 203, 21, "Nakagawa River Courts"),
    (0, 218, 216, 21, "Temple of Olympia Court"),
    (0, 242, 178, 21, "Miyabi Tennis Grove"),
    (70, 288, 119, 20, "Grass Court"),
    (77, 312, 105, 21, "Clay Court"),
    (75, 334, 109, 22, "Hard Court"),
    (8, 366, 54, 19, "Chika"),
    (8, 398, 64, 19, "Suzuki"),
    (9, 430, 35, 22, "Lily"),
    (8, 459, 44, 14, "Robot"),
    (7, 473, 49, 12, "Tennis"),
    (143, 462, 51, 19, "Anna"),
];

/// The confirmation screen: modes, setting names, offbeat rules, handicaps and play styles.
const CONFIRM: &[Label<'static>] = &[
    (2, 5, 107, 20, "Free Game"),
    (2, 37, 202, 23, "Hot Shots Challenge"),
    (1, 70, 143, 22, "Training Mode"),
    (2, 102, 162, 19, "Fun Time Tennis"),
    (2, 133, 124, 20, "Select Court"),
    (1, 165, 185, 23, "Advanced Settings"),
    (2, 197, 132, 23, "Set Handicap"),
    (2, 229, 132, 20, "Offbeat Rules"),
    (2, 261, 139, 23, "Select Umpire"),
    (27, 298, 145, 18, "Irregular Bounce"),
    (45, 330, 108, 18, "Slow Motion"),
    (0, 364, 198, 14, "Overhead Camera Always on Same Side"),
    (64, 465, 175, 18, "All Offbeat Rules On"),
    (361, 488, 31, 19, "ON"),
    (405, 488, 39, 19, "OFF"),
    (256, 6, 64, 19, "Games"),
    (256, 38, 43, 19, "Sets"),
    (256, 70, 117, 19, "Select Court"),
    (256, 102, 125, 22, "Select Umpire"),
    (255, 134, 125, 22, "Advanced Settings"),
    (231, 166, 161, 20, "Start the match!"),
    (290, 198, 69, 22, "Umpire"),
    (249, 229, 125, 23, "Settings OK!"),
    (256, 262, 125, 22, "Set Handicap"),
    (249, 294, 125, 22, "Ready to go!"),
    (290, 326, 157, 22, "Irregular Bounce"),
    (289, 358, 116, 19, "Slow Motion"),
    (385, 6, 125, 19, "Offbeat Rules"),
    (409, 38, 50, 19, "None"),
    (483, 38, 27, 19, "1P"),
    (388, 70, 88, 22, "Weight x 1"),
    (482, 70, 28, 19, "2P"),
    (388, 102, 90, 22, "Weight x 2"),
    (482, 102, 28, 19, "3P"),
    (388, 134, 90, 22, "Weight x 3"),
    (482, 134, 28, 19, "4P"),
    (419, 172, 74, 16, "All-round"),
    (423, 196, 66, 16, "Baseline"),
    (416, 220, 81, 16, "Net Player"),
    (416, 243, 81, 17, "Big Server"),
];

/// The rank titles.
const RANKS: &[Label<'static>] = &[
    (6, 2, 82, 18, "Tennis Lv 5"),
    (6, 26, 82, 18, "Tennis Lv 4"),
    (6, 50, 82, 18, "Tennis Lv 3"),
    (6, 74, 82, 18, "Tennis Lv 2"),
    (6, 98, 80, 18, "Tennis Lv 1"),
    (4, 122, 86, 21, "Tennis Hopeful"),
    (5, 146, 84, 18, "Tennis Tier II"),
    (3, 170, 88, 18, "Tennis Tier III"),
    (2, 194, 91, 18, "Tennis Tier IV"),
    (98, 2, 93, 18, "Tennis Tier V"),
    (101, 26, 87, 18, "Tennis Master"),
    (103, 50, 82, 18, "Tennis Elite"),
    (101, 74, 86, 18, "Tennis Guru"),
    (99, 98, 90, 21, "Tennis Legend"),
    (102, 122, 85, 18, "Tennis Hero"),
    (103, 146, 82, 21, "Tennis King"),
    (99, 170, 90, 21, "Tennis Superstar"),
];

/// The pause and result dialogs' rows.
const DIALOG: &[Label<'static>] = &[
    (78, 3, 101, 24, "Resume"),
    (4, 35, 249, 24, "Return to the Menu"),
    (2, 66, 253, 29, "Retry with Same Settings"),
    (41, 98, 175, 25, "Save and Exit"),
    (75, 131, 111, 24, "Rematch"),
    (96, 163, 70, 28, "Retry"),
    (12, 195, 236, 24, "Resume the Match"),
];

/// The pause panel's heading, set count and words.
const PAUSED: &[Label<'static>] = &[
    (1, 7, 62, 19, "Paused"),
    (2, 43, 10, 19, "1"),
    (17, 43, 14, 19, "2"),
    (33, 43, 14, 20, "3"),
    (48, 43, 15, 19, "4"),
    (65, 43, 14, 20, "5"),
    (81, 42, 15, 21, "6"),
    (1, 67, 43, 19, "sets"),
    (1, 92, 59, 19, "games"),
];

const TEAMS: &[Label<'static>] = &[(0, 4, 95, 24, "A Team"), (2, 36, 93, 22, "B Team")];

/// The result stats: digits, units, the stat names and the players.
const STATS: &[Label<'static>] = &[
    (1, 5, 15, 18, "0"),
    (18, 5, 11, 17, "1"),
    (34, 5, 13, 17, "2"),
    (49, 5, 14, 18, "3"),
    (64, 5, 15, 17, "4"),
    (81, 5, 14, 18, "5"),
    (97, 5, 14, 18, "6"),
    (113, 5, 14, 17, "7"),
    (129, 5, 14, 18, "8"),
    (145, 5, 15, 18, "9"),
    (162, 15, 8, 7, "."),
    (4, 27, 39, 17, "mph"),
    (75, 29, 17, 15, "%"),
    (92, 28, 23, 16, "pts"),
    (38, 69, 85, 29, "Stats"),
    (23, 107, 115, 18, "Fastest Serve"),
    (32, 131, 96, 18, "Points Won"),
    (24, 155, 113, 18, "Missed Shots"),
    (22, 179, 117, 18, "Double Faults"),
    (26, 203, 108, 18, "Service Aces"),
    (24, 228, 135, 18, "Sweet Spot Hits"),
    (17, 251, 126, 20, "Net Play Rate"),
    (14, 275, 133, 18, "Forehand Shots"),
    (3, 299, 155, 19, "Match Rating/100 pts"),
    (3, 323, 99, 19, "Longest Rally"),
    (1, 348, 87, 18, "Match Time :"),
    (1, 395, 32, 19, "1P"),
    (0, 418, 33, 20, "2P"),
    (0, 442, 33, 20, "3P"),
    (0, 467, 33, 19, "4P"),
    (0, 490, 47, 20, "COM"),
];

/// The result's key hints.
const RESULT_KEYS: &[Label<'static>] = &[
    (2, 2, 142, 20, "Score Screen"),
    (1, 26, 136, 20, "Stats Screen"),
    (2, 50, 96, 20, "Continue"),
    (1, 74, 72, 20, "Cancel"),
    (2, 99, 55, 19, "Done"),
];

const OPTION_00: &[Label<'static>] = &[
    (235, 5, 71, 21, "Sound"),
    (429, 5, 39, 23, "ON"),
    (239, 37, 65, 21, "Music"),
    (423, 37, 51, 23, "OFF"),
    (194, 69, 154, 25, "Short Replays"),
    (408, 71, 83, 23, "Opening"),
    (233, 101, 77, 21, "Movies"),
    (416, 104, 69, 22, "Ending"),
    (218, 133, 107, 24, "Load Tips"),
    (422, 136, 57, 19, "Mono"),
    (220, 165, 101, 21, "Vibration"),
    (417, 167, 67, 20, "Stereo"),
    (219, 197, 103, 21, "Autosave"),
    (201, 229, 140, 21, "Shot Markers"),
    (399, 231, 104, 23, "Infrequent"),
    (27, 238, 75, 20, "Default"),
    (174, 261, 194, 21, "Overhead Camera"),
    (27, 271, 75, 22, "English"),
    (405, 264, 91, 22, "Frequent"),
    (231, 293, 80, 21, "Voices"),
    (29, 303, 71, 19, "French"),
    (32, 335, 64, 19, "Italian"),
    (24, 366, 79, 20, "German"),
    (23, 398, 82, 23, "Spanish"),
    (16, 431, 96, 22, "Japanese"),
];

/// `data` (a TIM2 sheet called `name`) as an image: rebuilt if it's one of ours, else the disc's.
pub fn image(images: &mut Assets<Image>, data: &[u8], name: &str) -> Handle<Image> {
    let h = sheet_image(images, data, name);
    crate::asset_use::source(h.id(), data, name);
    h
}

fn sheet_image(images: &mut Assets<Image>, data: &[u8], name: &str) -> Handle<Image> {
    // match_stats names `info` by its folder ("/2d/info.tm2")
    let name = name.trim_end_matches(".tm2").rsplit('/').next().unwrap_or_default().to_ascii_lowercase();
    let off = OFF.load(Ordering::Relaxed);
    let wall = name.starts_with("wall_") && !off;
    let sheet = SHEETS.iter().find(|s| s.name == name).filter(|_| !off);
    // P50: while a mode starts, handed off (`crate::jobs`; `land` puts it in when it's done); P57: the disc's own
    // sheets and the walls too, and the decode with them (a menu start's ~50 decodes were ~50 ms of one frame on
    // desktop, more in the browser), unless a texture replacement may stand in for a disc sheet (`textures::add`)
    if DEFER.get() && (wall || sheet.is_some() || !crate::textures::on()) {
        let data = data.to_vec();
        let job = match sheet {
            // B409: a step at a time, so the browser draws frames in between
            Some(sheet) => {
                let (name, mut steps) = (name.clone(), None);
                crate::jobs::spawn_steps(move || steps.get_or_insert_with(|| Steps::new(&data, &name, sheet)).step())
            }
            None if wall => crate::jobs::spawn(move || wall_image(&data)),
            None => crate::jobs::spawn(move || disc_image(&data)),
        };
        if let Some(job) = job {
            let handle = images.reserve_handle();
            BUILDING.lock().unwrap().push((handle.clone(), job));
            if wall || sheet.is_some() {
                crate::textures::ours(handle.id(), Vec2::splat(K as f32));
            }
            return handle;
        }
    }
    let Some(sheet) = sheet else {
        if wall {
            let handle = images.add(wall_image(data));
            crate::textures::ours(handle.id(), Vec2::splat(K as f32));
            return handle;
        }
        return super::panel::image(images, data);
    };
    let handle = images.add(rebuilt(data, &name, sheet));
    crate::textures::ours(handle.id(), Vec2::splat(K as f32));
    handle
}

/// A menu wall drawn again at `K`× ([`grid`]).
fn wall_image(data: &[u8]) -> Image {
    let pic = tim2::decode_alpha8(data).expect("TIM2").remove(0);
    let (w, h) = (pic.width as usize, pic.height as usize);
    let rgba = grid(&pic.rgba, w, h);
    Image::new(Extent3d { width: w as u32 * K, height: h as u32 * K, depth_or_array_layers: 1 }, TextureDimension::D2, rgba, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD)
}

/// A disc sheet as it is ([`super::panel::image`] with no replacement to look up).
fn disc_image(data: &[u8]) -> Image {
    let pic = tim2::decode_alpha8(data).expect("TIM2").remove(0);
    Image::new(Extent3d { width: pic.width, height: pic.height, depth_or_array_layers: 1 }, TextureDimension::D2, pic.rgba, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD)
}

thread_local! {
    /// Set while a mode's start runs on this thread (`defer`).
    static DEFER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// P50: rebuilt sheets still being drawn on the async pool, each with the handle it goes in.
static BUILDING: std::sync::Mutex<Vec<(Handle<Image>, crate::jobs::Job<Image>)>> = std::sync::Mutex::new(Vec::new());

/// P50: sheets [`image`] rebuilds on this thread from now (`on`) are drawn on the async pool, not right away: a mode's
/// start (`mode.rs`) has them drawn on the other cores (a frame's budget at a time in the browser, B409) while it goes on,
/// and waits for them ([`building`]). A whole menu's took 0.5 s of one frame on desktop.
pub(crate) fn defer(on: bool) {
    DEFER.set(on);
}

/// Whether any deferred sheet isn't in yet.
pub(crate) fn building() -> bool {
    !BUILDING.lock().unwrap().is_empty()
}

/// Puts each deferred sheet that's done in its image.
pub(crate) fn land(images: Option<ResMut<Assets<Image>>>) {
    let Some(mut images) = images else { return };
    BUILDING.lock().unwrap().retain_mut(|(handle, task)| {
        let Some(img) = task.poll() else { return true };
        // its mode already gone: nothing to put it in
        let _ = images.insert(handle.id(), img);
        false
    });
}

/// Sheet `name` (`data`, one of ours) rebuilt.
fn rebuilt(data: &[u8], name: &str, sheet: &'static Sheet) -> Image {
    let mut steps = Steps::new(data, name, sheet);
    loop {
        if let Some(img) = steps.step() {
            return img;
        }
    }
}

/// B409: sheet `name` rebuilt a step at a time ([`Steps::step`]), each a few ms: the browser draws frames between
/// them (`jobs`), where one whole sheet was 0.2-0.3 s of a frame on desktop and over a second on a phone.
struct Steps {
    name: String,
    sheet: &'static Sheet,
    redo: Rebuild,
    /// The stamps drawn so far (their cells cleared first).
    stamps: Option<usize>,
    /// The finished sheet's pixels, unpremultiplied so far.
    pixels: Option<Vec<u8>>,
}

impl Steps {
    fn new(data: &[u8], name: &str, sheet: &'static Sheet) -> Steps {
        let pic = tim2::decode_alpha8(data).expect("TIM2").remove(0);
        let (w, h) = (pic.width as usize, pic.height as usize);
        let mut labels: Vec<(u16, u16, u16, u16, String)> = sheet.labels.iter().map(|&(x, y, lw, lh, t)| (x, y, lw, lh, t.to_string())).collect();
        let mut shadow = false;
        if let Some(words) = sheet.title {
            let (x, y, lw, lh) = title_box(&pic.rgba, w, h);
            labels.push((x, y, lw, lh, words.to_string()));
            shadow = true;
        }
        if name == "word" {
            labels.extend(glyph_boxes(&pic.rgba, w));
        }
        let pills = PILLS.iter().find(|p| p.0 == name).map_or(&[][..], |p| p.1);
        let redo = Rebuild::new(w, h, &pic.rgba, pills, labels, if name == "word" { BOLD } else { HEAVY }, shadow);
        Steps { name: name.to_string(), sheet, redo, stamps: None, pixels: None }
    }

    /// Does the next piece; the image once it's all done.
    fn step(&mut self) -> Option<Image> {
        if !self.redo.step() {
            return None;
        }
        let out = &mut self.redo.out;
        let stamps = self.sheet.stamps;
        if !stamps.is_empty() && self.stamps.is_none_or(|n| n < stamps.len()) {
            let Some(n) = self.stamps else {
                // a ring's outer pixels lie a little past its cell: clear round every cell before any is stamped
                for &((x, y, w, h), ..) in stamps {
                    clear(out, [x as f32 - 4.0, y as f32 - 4.0, w as f32 + 8.0, h as f32 + 8.0]);
                }
                self.stamps = Some(0);
                return None;
            };
            let font = ab_glyph::FontRef::try_from_slice(HEAVY).expect("menu font");
            let ((x, y, w, h), words, l) = stamps[n];
            stamp(out, &font, [x as f32, y as f32, w as f32, h as f32], words, &l);
            self.stamps = Some(n + 1);
            return None;
        }
        let Some(px) = &mut self.pixels else {
            if let Some(icons) = self.sheet.icons {
                icons(out);
            }
            if let Ok(dir) = std::env::var("HST_MENU_ART") {
                let _ = image::save_buffer(format!("{dir}/{}.png", self.name), out.data(), out.width(), out.height(), image::ColorType::Rgba8);
            }
            self.pixels = Some(Vec::with_capacity(out.data().len()));
            return None;
        };
        let all = out.pixels();
        let at = px.len() / 4;
        let to = (at + BAND).min(all.len());
        // P57: a clear pixel (most of a sheet) is clear either way, without demultiply's divisions
        px.extend(all[at..to].iter().flat_map(|c| match c.alpha() {
            0 => [0; 4],
            _ => {
                let c = c.demultiply();
                [c.red(), c.green(), c.blue(), c.alpha()]
            }
        }));
        if to < all.len() {
            return None;
        }
        let (w, h) = (out.width(), out.height());
        Some(Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, self.pixels.take().unwrap(), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD))
    }
}

/// B409: output pixels a step of [`Steps`] / [`Rebuild`] goes through (scaled up or unpremultiplied).
const BAND: usize = 1 << 16;

/// A menu wall (graph paper: a dark line every 16 texels, a faint one between) drawn again clean at `K`×: a column or
/// row that is mostly one line colour on the disc is that line, the hand-drawn wobble and dashes are dropped. Each
/// line stays one disc px wide, so the tile keeps its look but its edges stay sharp when the screen magnifies it.
fn grid(rgba: &[u8], w: usize, h: usize) -> Vec<u8> {
    let px = |x: usize, y: usize| -> [u8; 4] { rgba[(y * w + x) * 4..][..4].try_into().unwrap() };
    let mut count = std::collections::HashMap::<[u8; 4], usize>::new();
    (0..h).for_each(|y| (0..w).for_each(|x| *count.entry(px(x, y)).or_default() += 1));
    let bg = *count.iter().max_by_key(|c| c.1).unwrap().0;
    let lum = |c: [u8; 4]| c[0] as u32 + c[1] as u32 + c[2] as u32;
    // a line's colour: the commonest non-paper colour along it, if it covers most of it; the darker one wins a crossing
    let line = |cells: Vec<[u8; 4]>| {
        let n = cells.len();
        let mut c = std::collections::HashMap::<[u8; 4], usize>::new();
        cells.into_iter().filter(|&p| p != bg).for_each(|p| *c.entry(p).or_default() += 1);
        c.into_iter().filter(|&(_, k)| k * 2 > n).map(|(p, _)| p).min_by_key(|&p| lum(p))
    };
    let cols: Vec<_> = (0..w).map(|x| line((0..h).map(|y| px(x, y)).collect())).collect();
    let rows: Vec<_> = (0..h).map(|y| line((0..w).map(|x| px(x, y)).collect())).collect();
    let k = K as usize;
    let mut out = Vec::with_capacity(w * h * k * k * 4);
    for y in 0..h * k {
        for x in 0..w * k {
            let c = [cols[x / k], rows[y / k]].into_iter().flatten().min_by_key(|&p| lum(p)).unwrap_or(bg);
            out.extend_from_slice(&c);
        }
    }
    out
}

/// A sheet the port made itself from disc glyphs (`pause::label`'s row, `w`×`h`), its one word `s` set again at `K`×
/// round its ink; `None` with `--original`.
pub(crate) fn relabel(images: &mut Assets<Image>, w: usize, h: usize, rgba: &[u8], s: &str) -> Option<Handle<Image>> {
    if OFF.load(Ordering::Relaxed) {
        return None;
    }
    let ink: Vec<(usize, usize)> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| rgba[(y * w + x) * 4 + 3] > 0x20).collect();
    let (x0, y0) = (ink.iter().map(|p| p.0).min()?, ink.iter().map(|p| p.1).min()?);
    let (x1, y1) = (ink.iter().map(|p| p.0).max()? + 1, ink.iter().map(|p| p.1).max()? + 1);
    let out = rebuild(w, h, rgba, &[], &[(x0 as u16, y0 as u16, (x1 - x0) as u16, (y1 - y0) as u16, s)], HEAVY, false);
    let (w, h) = (out.width(), out.height());
    let handle = images.add(Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, unpremultiply(out), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD));
    crate::textures::ours(handle.id(), Vec2::splat(K as f32));
    Some(handle)
}

pub(crate) fn unpremultiply(p: Pixmap) -> Vec<u8> {
    p.pixels().iter().map(|c| c.demultiply()).flat_map(|c| [c.red(), c.green(), c.blue(), c.alpha()]).collect()
}

/// The sheet at `K`×: labels erased and set again, the rest snapped sharp.
fn rebuild(w: usize, h: usize, rgba8: &[u8], pills: &[(u16, u16, u16, u16)], labels: &[Label], font: &'static [u8], shadow: bool) -> Pixmap {
    let labels = labels.iter().map(|&(x, y, lw, lh, t)| (x, y, lw, lh, t.to_string())).collect();
    let mut r = Rebuild::new(w, h, rgba8, pills, labels, font, shadow);
    while !r.step() {}
    r.out
}

/// [`rebuild`] a step at a time (B409): the disc picture with its labels erased, then scaled up a band of rows a step,
/// the pills, then a label a step.
struct Rebuild {
    w: usize,
    h: usize,
    /// The erased picture, premultiplied, so a colour doesn't bleed out of a transparent neighbour.
    pm: Vec<[f32; 4]>,
    pills: Vec<(u16, u16, u16, u16, [Vec<u8>; 3])>,
    labels: Vec<(u16, u16, u16, u16, String)>,
    styles: Vec<Style>,
    font: &'static [u8],
    shadow: bool,
    out: Pixmap,
    /// Output rows scaled up so far, then labels set (past the rows).
    at: usize,
}

impl Rebuild {
    fn new(w: usize, h: usize, rgba8: &[u8], pills: &[(u16, u16, u16, u16)], labels: Vec<(u16, u16, u16, u16, String)>, font: &'static [u8], shadow: bool) -> Rebuild {
        let mut rgba: Vec<[f32; 4]> = rgba8.chunks(4).map(|c| [c[0], c[1], c[2], c[3]].map(|v| v as f32 / 255.0)).collect();
        let styles: Vec<Style> = labels.iter().map(|&(x, y, lw, lh, ref t)| Style::sample(&rgba, w, h, &(x, y, lw, lh, t))).collect();
        for (&(x, y, lw, lh, ref t), s) in labels.iter().zip(&styles) {
            // an outline's faint outer pixel lies just past the ink box (left behind, it reads as a cut-off outline round
            // the new one); a title's old shadow stands 2 px down and right of its words
            let m = (s.width > 0.0) as u16;
            let (x, y) = (x.saturating_sub(m), y.saturating_sub(m));
            let pad = 2 * m + if shadow { 3 } else { 0 };
            erase(&mut rgba, w, h, &(x, y, (lw + pad).min(w as u16 - x), (lh + pad).min(h as u16 - y), t));
        }
        // colours from the disc bar: rim and ring at the top middle, the fill halfway down left of the words
        let pills = pills
            .iter()
            .map(|&(x, y, pw, ph)| {
                let at = |dx: u16, dy: u16| rgba8[((y + dy) as usize * w + (x + dx) as usize) * 4..][..3].to_vec();
                (x, y, pw, ph, [at(pw / 2, 0), at(pw / 2, 2), at(20, ph / 2)])
            })
            .collect();
        let pm = rgba.iter().map(|c| [c[0] * c[3], c[1] * c[3], c[2] * c[3], c[3]]).collect();
        let out = Pixmap::new(w as u32 * K, h as u32 * K).expect("sheet size");
        Rebuild { w, h, pm, pills, labels, styles, font, shadow, out, at: 0 }
    }

    /// Does the next piece; true once it's all done.
    fn step(&mut self) -> bool {
        let oh = self.out.height() as usize;
        if self.at < oh {
            let to = (self.at + (BAND / self.out.width() as usize).max(1)).min(oh);
            upscale(&self.pm, self.w, self.h, &mut self.out, self.at..to);
            self.at = to;
            if to == oh {
                for (x, y, pw, ph, cols) in std::mem::take(&mut self.pills) {
                    pill(&mut self.out, x, y, pw, ph, cols);
                }
            }
            return false;
        }
        let Some(&(x, y, w, h, ref t)) = self.labels.get(self.at - oh) else { return true };
        let font = ab_glyph::FontRef::try_from_slice(self.font).expect("menu font");
        label(&mut self.out, &font, &(x, y, w, h, t), &self.styles[self.at - oh], self.shadow);
        self.at += 1;
        self.at - oh == self.labels.len()
    }
}

/// A stadium in `cols` (rim, ring, fill), outer edge on the disc box, 2 disc px each for the rim and ring.
fn pill(out: &mut Pixmap, x: u16, y: u16, w: u16, h: u16, cols: [Vec<u8>; 3]) {
    let k = K as f32;
    let (x0, y0, x1, y1) = (x as f32 * k, y as f32 * k, (x + w) as f32 * k, (y + h) as f32 * k);
    // the box's pixels are the bar's own: clear them so the anti-aliased round ends blend onto nothing
    let ow = out.width();
    for py in y0 as u32..y1 as u32 {
        out.pixels_mut()[(py * ow + x0 as u32) as usize..(py * ow + x1 as u32) as usize].fill(tiny_skia::PremultipliedColorU8::TRANSPARENT);
    }
    for (i, col) in cols.iter().enumerate() {
        let d = 2.0 * k * i as f32;
        let mut paint = Paint { anti_alias: true, ..Paint::default() };
        paint.set_color_rgba8(col[0], col[1], col[2], 255);
        out.fill_path(&stadium(x0 + d, y0 + d, x1 - d, y1 - d), &paint, FillRule::Winding, Transform::identity(), None);
    }
}

/// A box with half-round ends.
fn stadium(l: f32, t: f32, rr: f32, b: f32) -> tiny_skia::Path {
    let r = (b - t) / 2.0;
    // quarter circles as cubics
    let c = r * 0.5523;
    let mut pb = PathBuilder::new();
    pb.move_to(l + r, t);
    pb.line_to(rr - r, t);
    pb.cubic_to(rr - r + c, t, rr, t + r - c, rr, t + r);
    pb.cubic_to(rr, b - r + c, rr - r + c, b, rr - r, b);
    pb.line_to(l + r, b);
    pb.cubic_to(l + r - c, b, l, b - r + c, l, b - r);
    pb.cubic_to(l, t + r - c, l + r - c, t, l + r, t);
    pb.close();
    pb.finish().expect("stadium")
}

/// Our own button pill and box, nothing from the disc: three white stadiums 64×40 apart (the whole pill 1..63 × 1..39,
/// then inset 2 and 4 px), then three rounded boxes 64×64 apart from y [`BOX_Y`] (radius 15: the rim's band, the ring's
/// band 2 px in, the fill 4 px in, each band alone so a fill can be see-through), `K`× sharp, for a button's or a
/// panel's rim, ring and fill each tinted its own colour.
pub fn pill_layers(images: &mut Assets<Image>) -> Handle<Image> {
    let k = K as f32;
    let mut out = Pixmap::new(64 * K, (ICON_Y as u32 + 72) * K).expect("pill size");
    let mut paint = Paint { anti_alias: true, ..Paint::default() };
    paint.set_color_rgba8(255, 255, 255, 255);
    for i in 0..3 {
        let (d, y) = (2.0 * i as f32, 40.0 * i as f32);
        out.fill_path(&stadium((1.0 + d) * k, (y + 1.0 + d) * k, (63.0 - d) * k, (y + 39.0 - d) * k), &paint, FillRule::Winding, Transform::identity(), None);
    }
    for i in 0..3 {
        let y = BOX_Y + 64.0 * i as f32;
        let rect = |d: f32| rounded((1.0 + d) * k, (y + 1.0 + d) * k, (63.0 - d) * k, (y + 63.0 - d) * k, (15.0 - d).max(1.0) * k);
        let mut pb = PathBuilder::new();
        pb.push_path(&rect(2.0 * i as f32));
        if i < 2 {
            // the band's hole reaches half a pixel under the next band in, so no seam shows between them
            pb.push_path(&rect(2.0 * i as f32 + 2.5));
        }
        out.fill_path(&pb.finish().expect("box"), &paint, FillRule::EvenOdd, Transform::identity(), None);
    }
    meter_icons(&mut out);
    let (w, h) = (out.width(), out.height());
    let handle = images.add(Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, unpremultiply(out), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD));
    crate::textures::ours(handle.id(), Vec2::splat(k));
    handle
}

/// Where [`pill_layers`]' boxes start on the sheet (disc px).
const BOX_Y: f32 = 120.0;

/// B141a: our icons on [`pill_layers`]' sheet below the boxes (disc px): the confirm screen's meter balls, lit and
/// not, and its bounce and speed meters' icons.
const ICON_Y: f32 = BOX_Y + 192.0;
pub const BALL: [f32; 4] = [0.0, ICON_Y, 24.0, 24.0];
pub const BALL_OFF: [f32; 4] = [24.0, ICON_Y, 24.0, 24.0];
pub const BOUNCE: [f32; 4] = [0.0, ICON_Y + 24.0, 45.0, 22.0];
pub const SPEED: [f32; 4] = [0.0, ICON_Y + 48.0, 46.0, 17.0];

/// The meter icons at [`ICON_Y`].
fn meter_icons(out: &mut Pixmap) {
    let y = ICON_Y;
    ball(out, 12.0, y + 12.0, 10.5, [215, 235, 60], [80, 95, 15]);
    ball(out, 36.0, y + 12.0, 10.5, [72, 76, 52], [30, 32, 22]);
    // bounce: the ball's dotted path down, up off the court and down again
    let y = y + 24.0;
    let mut pb = PathBuilder::new();
    pb.move_to(2.0, y + 3.0);
    pb.quad_to(12.0, y + 4.0, 17.0, y + 19.0);
    pb.quad_to(22.0, y + 3.0, 31.0, y + 11.0);
    let path = pb.finish().expect("bounce");
    let dash = tiny_skia::StrokeDash::new(vec![2.5, 2.0], 0.0);
    lines(out, &path, 1.6, dash);
    ball(out, 37.5, y + 13.0, 6.5, [215, 235, 60], [80, 95, 15]);
    // speed: three streaks behind the ball
    let y = y + 24.0;
    let mut pb = PathBuilder::new();
    for (i, (x0, x1)) in [(12.0, 28.0), (2.0, 26.0), (12.0, 28.0)].into_iter().enumerate() {
        pb.move_to(x0, y + 3.5 + 5.0 * i as f32);
        pb.line_to(x1, y + 3.5 + 5.0 * i as f32);
    }
    lines(out, &pb.finish().expect("speed"), 1.8, None);
    ball(out, 37.5, y + 8.5, 6.5, [215, 235, 60], [80, 95, 15]);
}

/// White strokes with a dark edge, disc px.
fn lines(out: &mut Pixmap, path: &tiny_skia::Path, w: f32, dash: Option<tiny_skia::StrokeDash>) {
    let t = Transform::from_scale(K as f32, K as f32);
    let mut paint = Paint { anti_alias: true, ..Paint::default() };
    for (col, w) in [([30, 30, 30], w + 1.6), ([245, 245, 245], w)] {
        paint.set_color_rgba8(col[0], col[1], col[2], 255);
        let stroke = Stroke { width: w, line_cap: LineCap::Round, dash: dash.clone(), ..Stroke::default() };
        out.stroke_path(path, &paint, &stroke, t, None);
    }
}

/// A tennis ball: `fill` with a darker rim and two pale seams, disc px.
fn ball(out: &mut Pixmap, cx: f32, cy: f32, r: f32, fill: [u8; 3], rim: [u8; 3]) {
    let t = Transform::from_scale(K as f32, K as f32);
    fill_rgb(out, &PathBuilder::from_circle(cx, cy, r).expect("ball"), rim, t);
    fill_rgb(out, &PathBuilder::from_circle(cx, cy, r - 1.2).expect("ball"), fill, t);
    let mut pb = PathBuilder::new();
    pb.move_to(cx - r * 0.85, cy - r * 0.45);
    pb.quad_to(cx - r * 0.1, cy, cx - r * 0.85, cy + r * 0.45);
    pb.move_to(cx + r * 0.85, cy - r * 0.45);
    pb.quad_to(cx + r * 0.1, cy, cx + r * 0.85, cy + r * 0.45);
    let seam = fill.map(|c| c.saturating_add(70));
    stroke_rgb(out, &pb.finish().expect("seam"), seam, r * 0.13, t);
}

pub(crate) fn fill_rgb(out: &mut Pixmap, path: &tiny_skia::Path, c: [u8; 3], t: Transform) {
    let mut paint = Paint { anti_alias: true, ..Paint::default() };
    paint.set_color_rgba8(c[0], c[1], c[2], 255);
    out.fill_path(path, &paint, FillRule::Winding, t, None);
}

pub(crate) fn stroke_rgb(out: &mut Pixmap, path: &tiny_skia::Path, c: [u8; 3], w: f32, t: Transform) {
    let mut paint = Paint { anti_alias: true, ..Paint::default() };
    paint.set_color_rgba8(c[0], c[1], c[2], 255);
    out.stroke_path(path, &paint, &Stroke { width: w, line_join: LineJoin::Round, line_cap: LineCap::Round, ..Stroke::default() }, t, None);
}

/// Clear `[x, y, w, h]` (disc px, clipped to the sheet).
fn clear(out: &mut Pixmap, [x, y, w, h]: [f32; 4]) {
    let k = K as f32;
    let (ow, oh) = (out.width() as f32, out.height() as f32);
    let (x0, y0, x1, y1) = ((x * k).clamp(0.0, ow) as u32, (y * k).clamp(0.0, oh) as u32, ((x + w) * k).clamp(0.0, ow) as u32, ((y + h) * k).clamp(0.0, oh) as u32);
    let stride = out.width();
    for py in y0..y1 {
        out.pixels_mut()[(py * stride + x0) as usize..(py * stride + x1) as usize].fill(tiny_skia::PremultipliedColorU8::TRANSPARENT);
    }
}

/// `words` in our font in `cell` (disc px, the ring's outer edge): its cap height fills the cell less the ring, the
/// width squeezed only if it doesn't fit, centred; ring, outline, then the fill's fade over the cap height.
fn stamp(out: &mut Pixmap, font: &ab_glyph::FontRef, [x, y, w, h]: [f32; 4], words: &str, l: &Look) {
    let (Some(path), Some(caps)) = (text_path(font, words), text_path(font, "H")) else { return };
    let k = K as f32;
    let m = l.ew + l.rw;
    let (bw, bh) = ((w - 2.0 * m) * k, (h - 2.0 * m) * k);
    let cap = -caps.bounds().top();
    // the slant leans the top right: room for it comes off the width
    let sy = bh / cap;
    let b = path.bounds();
    let sx = sy.min((bw - l.slant * bh) / b.width());
    let cx = (x + w / 2.0) * k;
    // font y is 0 at the baseline, −cap at the caps' top; the slant leans about the cell's middle
    let (top, uc) = ((y + m) * k, b.left() + b.width() / 2.0);
    let t = Transform::from_row(sx, 0.0, -l.slant * sy, sy, cx - uc * sx - l.slant * bh / 2.0, top + bh);
    let Some(path) = path.transform(t) else { return };
    let rgb = |c: [u8; 3]| SkColor::from_rgba8(c[0], c[1], c[2], 255);
    let mut paint = Paint { anti_alias: true, ..Paint::default() };
    for (c, wd) in [(l.ring, 2.0 * m * k), (l.edge, 2.0 * l.ew * k)] {
        if wd > 0.0 {
            paint.set_color(rgb(c));
            out.stroke_path(&path, &paint, &Stroke { width: wd, line_join: LineJoin::Round, line_cap: LineCap::Round, ..Stroke::default() }, Transform::identity(), None);
        }
    }
    let bot = top + bh;
    paint.shader = LinearGradient::new(Point::from_xy(0.0, top), Point::from_xy(0.0, bot), vec![GradientStop::new(0.0, rgb(l.top)), GradientStop::new(1.0, rgb(l.bottom))], SpreadMode::Pad, Transform::identity()).expect("gradient");
    out.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}

/// `info.tm2`'s buttons drawn ours (its left half, the plain boxes and Yes/No, stays the disc's): L1/L2/R1/R2, the
/// d-pad with each arm lit and plain, △ □ ○ ✕, the orange dot, START and SELECT plain and lit, the four arrows.
fn info_icons(out: &mut Pixmap) {
    clear(out, [144.0, 0.0, 112.0, 256.0]);
    let t = Transform::from_scale(K as f32, K as f32);
    let font = ab_glyph::FontRef::try_from_slice(HEAVY).expect("menu font");
    const DARK: [u8; 3] = [24, 24, 28];
    const BODY: [u8; 3] = [72, 74, 80];
    const LIT: [u8; 3] = [250, 160, 30];
    let white = look([255; 3], [215; 3], DARK, 0.0, 0.0);
    let white = Look { rw: 0.0, ..white };
    let label = Look { ew: 1.2, rw: 0.0, ..white };
    for (k, s) in ["L1", "L2", "R1", "R2"].into_iter().enumerate() {
        let y = 9.0 + 32.0 * k as f32;
        fill_rgb(out, &rounded(160.0, y, 192.0, y + 19.0, 6.0), DARK, t);
        fill_rgb(out, &rounded(161.5, y + 1.5, 190.5, y + 17.5, 4.5), BODY, t);
        stamp(out, &font, [166.0, y + 4.0, 20.0, 11.0], s, &white);
    }
    // the d-pad: lit arm k (up, left, right, down) at (208, 16 + 32k); plain at (160, 144)
    let pads = [(208.0, 16.0, Some(0)), (208.0, 48.0, Some(1)), (208.0, 80.0, Some(2)), (208.0, 112.0, Some(3)), (160.0, 144.0, None)];
    for (cx, cy, lit) in pads {
        let (a, l) = (4.0, 13.0);
        let mut pb = PathBuilder::new();
        for (px, py) in [(-a, -l), (a, -l), (a, -a), (l, -a), (l, a), (a, a), (a, l), (-a, l), (-a, a), (-l, a), (-l, -a), (-a, -a)] {
            if pb.is_empty() {
                pb.move_to(cx + px, cy + py);
            } else {
                pb.line_to(cx + px, cy + py);
            }
        }
        pb.close();
        let cross = pb.finish().expect("d-pad");
        stroke_rgb(out, &cross, DARK, 2.4, t);
        fill_rgb(out, &cross, BODY, t);
        if let Some(arm) = lit {
            let r = match arm {
                0 => [cx - a, cy - l, cx + a, cy - a],
                1 => [cx - l, cy - a, cx - a, cy + a],
                2 => [cx + a, cy - a, cx + l, cy + a],
                _ => [cx - a, cy + a, cx + a, cy + l],
            };
            let arm = rounded(r[0] + 0.6, r[1] + 0.6, r[2] - 0.6, r[3] - 0.6, 1.5);
            fill_rgb(out, &arm, LIT, t);
        }
    }
    // △ □ ○ ✕ in their PlayStation colours
    let face = [[70, 205, 165], [235, 130, 185], [240, 95, 85], [110, 160, 240]];
    for (k, col) in face.into_iter().enumerate() {
        let (cx, cy) = (240.0, 16.0 + 32.0 * k as f32);
        fill_rgb(out, &PathBuilder::from_circle(cx, cy, 12.0).expect("button"), DARK, t);
        fill_rgb(out, &PathBuilder::from_circle(cx, cy, 10.6).expect("button"), [58, 60, 66], t);
        let r = 5.5;
        let mut pb = PathBuilder::new();
        match k {
            0 => {
                pb.move_to(cx, cy - r);
                pb.line_to(cx + r * 0.95, cy + r * 0.6);
                pb.line_to(cx - r * 0.95, cy + r * 0.6);
                pb.close();
            }
            1 => pb.push_rect(tiny_skia::Rect::from_xywh(cx - r * 0.8, cy - r * 0.8, r * 1.6, r * 1.6).expect("square")),
            2 => pb.push_circle(cx, cy, r * 0.9),
            _ => {
                pb.move_to(cx - r * 0.8, cy - r * 0.8);
                pb.line_to(cx + r * 0.8, cy + r * 0.8);
                pb.move_to(cx + r * 0.8, cy - r * 0.8);
                pb.line_to(cx - r * 0.8, cy + r * 0.8);
            }
        }
        stroke_rgb(out, &pb.finish().expect("symbol"), col, 2.0, t);
    }
    fill_rgb(out, &PathBuilder::from_circle(160.0, 176.0, 13.5).expect("dot"), LIT, t);
    // START (a play triangle) and SELECT (a flat key), the lit ones rimmed orange, their names under them
    for (y, rim) in [(132.0, DARK), (162.0, LIT)] {
        let mut pb = PathBuilder::new();
        pb.move_to(188.0, y + 2.0);
        pb.line_to(212.0, y + 10.0);
        pb.line_to(188.0, y + 18.0);
        pb.close();
        let tri = pb.finish().expect("start");
        stroke_rgb(out, &tri, rim, 3.0, t);
        fill_rgb(out, &tri, BODY, t);
        stamp(out, &font, [183.0, y + 18.0, 35.0, 10.0], "START", &label);
    }
    for (y, rim) in [(198.0, DARK), (228.0, LIT)] {
        let key = rounded(186.0, y + 1.0, 214.0, y + 13.0, 3.0);
        stroke_rgb(out, &key, rim, 2.4, t);
        fill_rgb(out, &key, BODY, t);
        stamp(out, &font, [180.0, y + 14.0, 41.0, 10.0], "SELECT", &label);
    }
    // the arrows: right, left, down, up
    let arrows = [
        [(229.0, 133.0), (229.0, 155.0), (249.0, 144.0)],
        [(251.0, 165.0), (251.0, 187.0), (231.0, 176.0)],
        [(229.0, 197.0), (251.0, 197.0), (240.0, 217.0)],
        [(240.0, 231.0), (229.0, 251.0), (251.0, 251.0)],
    ];
    for tri in arrows {
        let mut pb = PathBuilder::new();
        pb.move_to(tri[0].0, tri[0].1);
        pb.line_to(tri[1].0, tri[1].1);
        pb.line_to(tri[2].0, tri[2].1);
        pb.close();
        let tri = pb.finish().expect("arrow");
        stroke_rgb(out, &tri, [70, 72, 78], 2.4, t);
        fill_rgb(out, &tri, [240, 240, 244], t);
    }
}

/// A box with rounded corners of radius `r`.
pub(crate) fn rounded(l: f32, t: f32, rr: f32, b: f32, r: f32) -> tiny_skia::Path {
    let c = r * 0.5523;
    let mut pb = PathBuilder::new();
    pb.move_to(l + r, t);
    pb.line_to(rr - r, t);
    pb.cubic_to(rr - r + c, t, rr, t + r - c, rr, t + r);
    pb.line_to(rr, b - r);
    pb.cubic_to(rr, b - r + c, rr - r + c, b, rr - r, b);
    pb.line_to(l + r, b);
    pb.cubic_to(l + r - c, b, l, b - r + c, l, b - r);
    pb.line_to(l, t + r);
    pb.cubic_to(l, t + r - c, l + r - c, t, l + r, t);
    pb.close();
    pb.finish().expect("rounded box")
}

/// [`pill_layers`]' slices on the sheet (disc px): layer `i`'s left end, middle and right end, 38 tall.
pub(crate) fn pill_slices(i: usize) -> [[f32; 4]; 3] {
    let y = 40.0 * i as f32 + 1.0;
    [[1.0, y, 19.0, 38.0], [20.0, y, 24.0, 38.0], [44.0, y, 19.0, 38.0]]
}

/// Pill layer `i` over `[x, y, w, h]`: (source, destination) for its left end, middle and right end, the ends `h / 2`
/// wide so they stay round at any height.
pub fn pill_quads(i: usize, [x, y, w, h]: [f32; 4]) -> [([f32; 4], [f32; 4]); 3] {
    let [l, m, r] = pill_slices(i);
    let e = (h / 2.0).min(w / 2.0);
    [(l, [x, y, e, h]), (m, [x + e, y, w - 2.0 * e, h]), (r, [x + w - e, y, e, h])]
}

/// Box layer `i` over `[x, y, w, h]` with `c`-px corners: (source, destination) for each of its nine slices.
pub fn box_quads(i: usize, [x, y, w, h]: [f32; 4], c: f32) -> [([f32; 4], [f32; 4]); 9] {
    let (u, v, s) = (1.0, BOX_Y + 64.0 * i as f32 + 1.0, 16.0);
    let cols = [(u, s, x, c), (u + s, 62.0 - 2.0 * s, x + c, w - 2.0 * c), (u + 62.0 - s, s, x + w - c, c)];
    let rows = [(v, s, y, c), (v + s, 62.0 - 2.0 * s, y + c, h - 2.0 * c), (v + 62.0 - s, s, y + h - c, c)];
    std::array::from_fn(|k| {
        let ((su, sw, dx, dw), (sv, sh, dy, dh)) = (cols[k % 3], rows[k / 3]);
        ([su, sv, sw, sh], [dx, dy, dw, dh])
    })
}

/// A label's colours, from its disc pixels: the bright fill per row, the dark outline and its width.
struct Style {
    rows: Vec<(f32, [f32; 3])>,
    edge: [f32; 3],
    width: f32,
}

fn luma(c: &[f32; 4]) -> f32 {
    0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
}

impl Style {
    fn sample(px: &[[f32; 4]], w: usize, h: usize, &(x, y, lw, lh, _): &Label) -> Style {
        let (x0, y0, x1, y1) = (x as usize, y as usize, (x + lw) as usize, (y + lh) as usize);
        let at = |x: usize, y: usize| px[y.min(h - 1) * w + x.min(w - 1)];
        let mut ls: Vec<f32> = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).map(|(x, y)| at(x, y)).filter(|c| c[3] > 0.6).map(|c| luma(&c)).collect();
        ls.sort_by(f32::total_cmp);
        let (lo, hi) = if ls.is_empty() { (0.0, 1.0) } else { (ls[ls.len() / 10], ls[ls.len() * 9 / 10]) };
        // a plain glyph (no outline) has no dark ring: everything opaque is fill
        let flat = hi - lo < 0.25;
        let bright = |c: &[f32; 4]| c[3] > 0.6 && (flat || luma(c) > lo + 0.6 * (hi - lo));
        let dark = |c: &[f32; 4]| c[3] > 0.6 && !flat && luma(c) < lo + 0.4 * (hi - lo);
        let mut rows = vec![];
        let (mut edge, mut nd, mut border) = ([0.0f32; 3], 0.0f32, 0.0f32);
        for y in y0..y1 {
            let (mut sum, mut n) = ([0.0; 3], 0.0);
            for x in x0..x1 {
                let c = at(x, y);
                if bright(&c) {
                    (0..3).for_each(|i| sum[i] += c[i]);
                    n += 1.0;
                    let nb = [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)];
                    if nb.iter().any(|&(u, v)| u >= w || v >= h || !bright(&at(u, v))) {
                        border += 1.0;
                    }
                } else if dark(&c) {
                    (0..3).for_each(|i| edge[i] += c[i]);
                    nd += 1.0;
                }
            }
            if n > 0.0 {
                rows.push((((y - y0) as f32 + 0.5) / (y1 - y0) as f32, sum.map(|s| s / n)));
            }
        }
        // under 10 px (the hand tags' words) the dark round the letters is the tag, not an outline
        let rows = if rows.is_empty() { vec![(0.5, [1.0; 3])] } else { rows };
        Style { rows, edge: if nd > 0.0 { edge.map(|e| e / nd) } else { [0.0; 3] }, width: if nd > 0.0 && lh >= 10 { (nd / border.max(1.0)).clamp(0.5, 4.0) } else { 0.0 } }
    }
}

/// Clear a label's box row by row, blending what's either side of it: nothing where it stood alone, the pill or the
/// title's stripes where it stood on them.
fn erase(px: &mut [[f32; 4]], w: usize, _h: usize, &(x, y, lw, lh, _): &Label) {
    let (l, r) = (x as usize, (x + lw) as usize);
    for y in y as usize..(y + lh) as usize {
        let a = if l > 0 { px[y * w + l - 1] } else { [0.0; 4] };
        let b = if r < w { px[y * w + r] } else { [0.0; 4] };
        for x in l..r {
            let t = (x + 1 - l) as f32 / (r - l + 1) as f32;
            px[y * w + x] = if a[3] < 0.5 && b[3] < 0.5 { [0.0; 4] } else { std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t) };
        }
    }
}

/// Where a title bar's words stand: the pixels that differ from their row's stripe, clear of the band's rule (its
/// white and black lines, the 7 rows above where each column turns transparent). Less the 2 px of the words' shadow,
/// which `rebuild` erases with them.
fn title_box(rgba: &[u8], w: usize, h: usize) -> (u16, u16, u16, u16) {
    let px = |x: usize, y: usize| &rgba[(y * w + x) * 4..][..4];
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for x in 0..w {
        let end = (0..h).find(|&y| px(x, y)[3] == 0).unwrap_or(h).saturating_sub(7);
        for y in 0..end {
            // the stripes repeat every 4 rows; the right end has nothing on them
            let bg = px(w - 8, y % 4);
            if (0..4).any(|i| (px(x, y)[i] as i32 - bg[i] as i32).abs() > 40) {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
            }
        }
    }
    (x0 as u16, y0 as u16, (x1 - x0).saturating_sub(2) as u16, (y1 - y0).saturating_sub(2) as u16)
}

/// `word.tm2`'s glyphs: each cell's ink box and its letter (`word_label::FONT`'s grid, 32 px cells from row 1).
fn glyph_boxes(rgba: &[u8], w: usize) -> Vec<(u16, u16, u16, u16, String)> {
    let mut out = vec![];
    for (row, chars) in super::word_label::FONT.iter().enumerate() {
        for (col, ch) in chars.chars().enumerate() {
            let (cx, cy) = (col * 32, 32 + row * 32);
            let ink: Vec<(usize, usize)> = (0..32).flat_map(|y| (0..32).map(move |x| (x, y))).filter(|&(x, y)| rgba[((cy + y) * w + cx + x) * 4 + 3] > 0x20).collect();
            let Some(x0) = ink.iter().map(|p| p.0).min() else { continue };
            let (x1, y0, y1) = (ink.iter().map(|p| p.0).max().unwrap(), ink.iter().map(|p| p.1).min().unwrap(), ink.iter().map(|p| p.1).max().unwrap());
            out.push(((cx + x0) as u16, (cy + y0) as u16, (x1 - x0 + 1) as u16, (y1 - y0 + 1) as u16, ch.to_string()));
        }
    }
    out
}

/// `K`× bilinear, then wherever the four disc pixels under it differ a lot, the edge between them snapped to a pixel.
/// P57: what depends only on the four disc pixels (their extremes, whether they're all one colour) is worked out once
/// per run of output pixels over the same four, and a fully transparent four is left clear: the same pixels as
/// working each one out from scratch, in a third of the time (most of a sheet is clear or flat; it was ~70% of a
/// sheet's rebuild, the browser's menu start waited seconds on it).
fn upscale(pm: &[[f32; 4]], w: usize, h: usize, out: &mut Pixmap, rows: std::ops::Range<usize>) {
    let ow = w * K as usize;
    let at = |x: isize, y: isize| pm[(y.clamp(0, h as isize - 1) as usize) * w + x.clamp(0, w as isize - 1) as usize];
    // the quad's two extremes by "ink" (alpha, then brightness): a hard edge between them is drawn hard
    let key = |p: &[f32; 4]| p[3] * 2.0 + luma(p);
    let data = out.pixels_mut();
    for oy in rows {
        let fy = (oy as f32 + 0.5) / K as f32 - 0.5;
        let (y0, ty) = (fy.floor() as isize, fy - fy.floor());
        // the four under the last pixel: their x, and (for a quad not all one colour) the extremes and their spread
        let mut last = None;
        let (mut q, mut lo, mut hi, mut range, mut flat) = ([[0.0f32; 4]; 4], [0.0f32; 4], [0.0f32; 4], 0.0, false);
        for ox in 0..ow {
            let fx = (ox as f32 + 0.5) / K as f32 - 0.5;
            let (x0, tx) = (fx.floor() as isize, fx - fx.floor());
            if last != Some(x0) {
                last = Some(x0);
                q = [at(x0, y0), at(x0 + 1, y0), at(x0, y0 + 1), at(x0 + 1, y0 + 1)];
                flat = q[1] == q[0] && q[2] == q[0] && q[3] == q[0];
                if !flat {
                    (lo, hi) = q.iter().fold((q[0], q[0]), |(a, b), p| (if key(p) < key(&a) { *p } else { a }, if key(p) > key(&b) { *p } else { b }));
                    range = key(&hi) - key(&lo);
                }
            }
            if flat && q[0] == [0.0; 4] {
                data[oy * ow + ox] = tiny_skia::PremultipliedColorU8::TRANSPARENT;
                continue;
            }
            let wt = [(1.0 - tx) * (1.0 - ty), tx * (1.0 - ty), (1.0 - tx) * ty, tx * ty];
            let mut c = [0.0f32; 4];
            for (p, k) in q.iter().zip(wt) {
                (0..4).for_each(|i| c[i] += p[i] * k);
            }
            if !flat && range > 0.6 {
                let t = ((key(&c) - key(&lo)) / range).clamp(0.0, 1.0);
                // a ramp one output pixel wide across the iso-line
                let s = ((t - 0.5) * K as f32 * range.min(1.5) + 0.5).clamp(0.0, 1.0);
                let s = s * s * (3.0 - 2.0 * s);
                (0..4).for_each(|i| c[i] = lo[i] + (hi[i] - lo[i]) * s);
            }
            let a = c[3].clamp(0.0, 1.0);
            let u = |v: f32| (v.clamp(0.0, a) * 255.0 + 0.5) as u8;
            data[oy * ow + ox] = tiny_skia::PremultipliedColorU8::from_rgba(u(c[0]), u(c[1]), u(c[2]), (a * 255.0 + 0.5) as u8).expect("premultiplied");
        }
    }
}

/// [`upscale`] working every output pixel out from scratch (before P57), to check it against.
#[cfg(test)]
fn plain_upscale(pm: &[[f32; 4]], w: usize, h: usize, out: &mut Pixmap, rows: std::ops::Range<usize>) {
    let ow = w * K as usize;
    let at = |x: isize, y: isize| pm[(y.clamp(0, h as isize - 1) as usize) * w + x.clamp(0, w as isize - 1) as usize];
    let data = out.pixels_mut();
    for oy in rows {
        let fy = (oy as f32 + 0.5) / K as f32 - 0.5;
        let (y0, ty) = (fy.floor() as isize, fy - fy.floor());
        for ox in 0..ow {
            let fx = (ox as f32 + 0.5) / K as f32 - 0.5;
            let (x0, tx) = (fx.floor() as isize, fx - fx.floor());
            let q = [at(x0, y0), at(x0 + 1, y0), at(x0, y0 + 1), at(x0 + 1, y0 + 1)];
            let wt = [(1.0 - tx) * (1.0 - ty), tx * (1.0 - ty), (1.0 - tx) * ty, tx * ty];
            let mut c = [0.0f32; 4];
            for (p, k) in q.iter().zip(wt) {
                (0..4).for_each(|i| c[i] += p[i] * k);
            }
            // the quad's two extremes by "ink" (alpha, then brightness): a hard edge between them is drawn hard
            let key = |p: &[f32; 4]| p[3] * 2.0 + luma(p);
            let (lo, hi) = q.iter().fold((q[0], q[0]), |(a, b), p| (if key(p) < key(&a) { *p } else { a }, if key(p) > key(&b) { *p } else { b }));
            let range = key(&hi) - key(&lo);
            if range > 0.6 {
                let t = ((key(&c) - key(&lo)) / range).clamp(0.0, 1.0);
                // a ramp one output pixel wide across the iso-line
                let s = ((t - 0.5) * K as f32 * range.min(1.5) + 0.5).clamp(0.0, 1.0);
                let s = s * s * (3.0 - 2.0 * s);
                (0..4).for_each(|i| c[i] = lo[i] + (hi[i] - lo[i]) * s);
            }
            let a = c[3].clamp(0.0, 1.0);
            let u = |v: f32| (v.clamp(0.0, a) * 255.0 + 0.5) as u8;
            data[oy * ow + ox] = tiny_skia::PremultipliedColorU8::from_rgba(u(c[0]), u(c[1]), u(c[2]), (a * 255.0 + 0.5) as u8).expect("premultiplied");
        }
    }
}

/// `s` as one path in font units (y down), and its advance.
pub(crate) fn text_path(font: &ab_glyph::FontRef, s: &str) -> Option<tiny_skia::Path> {
    use ab_glyph::{Font, OutlineCurve};
    let mut pb = PathBuilder::new();
    let (mut pen, mut prev) = (0.0, None);
    for ch in s.chars() {
        let id = font.glyph_id(ch);
        if let Some(p) = prev {
            pen += font.kern_unscaled(p, id);
        }
        if let Some(o) = font.outline(id) {
            let mut last: Option<ab_glyph::Point> = None;
            let pt = |p: ab_glyph::Point| (pen + p.x, -p.y);
            for c in &o.curves {
                let (start, end) = match *c {
                    OutlineCurve::Line(a, b) | OutlineCurve::Quad(a, _, b) | OutlineCurve::Cubic(a, _, _, b) => (a, b),
                };
                if last != Some(start) {
                    if last.is_some() {
                        pb.close();
                    }
                    let (x, y) = pt(start);
                    pb.move_to(x, y);
                }
                match *c {
                    OutlineCurve::Line(_, b) => {
                        let (x, y) = pt(b);
                        pb.line_to(x, y)
                    }
                    OutlineCurve::Quad(_, b, e) => {
                        let ((bx, by), (x, y)) = (pt(b), pt(e));
                        pb.quad_to(bx, by, x, y)
                    }
                    OutlineCurve::Cubic(_, b, c2, e) => {
                        let ((bx, by), (cx, cy), (x, y)) = (pt(b), pt(c2), pt(e));
                        pb.cubic_to(bx, by, cx, cy, x, y)
                    }
                }
                last = Some(end);
            }
            if last.is_some() {
                pb.close();
            }
        }
        pen += font.h_advance_unscaled(id);
        prev = Some(id);
    }
    pb.finish()
}

/// Set a label in its box at `K`×: the words' fill fitted inside the outline's width, outline then fill.
fn label(out: &mut Pixmap, font: &ab_glyph::FontRef, &(x, y, w, h, s): &Label, st: &Style, shadow: bool) {
    let Some(path) = text_path(font, s) else { return };
    let k = K as f32;
    let e = st.width * k;
    let b = path.bounds();
    let (bx, by, bw, bh) = (x as f32 * k + e, y as f32 * k + e, (w as f32 * k - 2.0 * e).max(1.0), (h as f32 * k - 2.0 * e).max(1.0));
    let t = Transform::from_row(bw / b.width(), 0.0, 0.0, bh / b.height(), bx - b.left() * bw / b.width(), by - b.top() * bh / b.height());
    let Some(path) = path.transform(t) else { return };
    let rgb = |c: [f32; 3]| SkColor::from_rgba(c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0), 1.0).expect("colour");
    let mut paint = Paint { anti_alias: true, ..Paint::default() };
    let stroke = Stroke { width: 2.0 * e.max(k * 0.5), line_join: LineJoin::Round, line_cap: LineCap::Round, ..Stroke::default() };
    if shadow {
        // the titles' soft grey shadow, down and right of the outline
        paint.set_color(SkColor::from_rgba(0.0, 0.0, 0.0, 0.3).expect("colour"));
        out.stroke_path(&path, &paint, &stroke, Transform::from_translate(1.5 * k, 1.5 * k), None);
        out.fill_path(&path, &paint, FillRule::Winding, Transform::from_translate(1.5 * k, 1.5 * k), None);
    }
    if e > 0.0 {
        paint.set_color(rgb(st.edge));
        out.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
    let (top, bot) = (y as f32 * k, (y + h) as f32 * k);
    let stops: Vec<GradientStop> = st.rows.iter().map(|&(t, c)| GradientStop::new(t, rgb(c))).collect();
    paint.shader = if stops.len() > 1 {
        LinearGradient::new(Point::from_xy(0.0, top), Point::from_xy(0.0, bot), stops, SpreadMode::Pad, Transform::identity()).expect("gradient")
    } else {
        tiny_skia::Shader::SolidColor(rgb(st.rows[0].1))
    };
    out.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A wall's grid lines survive at `K`×, its stray wobble pixels don't.
    /// P50: a sheet rebuilt on the async pool while a start defers them lands the same as one drawn right away, and
    /// only then is nothing left building.
    #[test]
    fn a_deferred_sheet_lands_the_same() {
        let Ok(mut iso) = hst_data::iso::Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let arc_data = iso.read("MENU/MENU04.XB0").expect("menu archive");
        let arc = hst_data::xb::Archive::parse(&arc_data).expect("xb");
        let word = arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("word.tm2")).expect("word.tm2")).expect("bytes");
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let mut images = Assets::<Image>::default();
        let now = image(&mut images, &word, "word");
        defer(true);
        let later = image(&mut images, &word, "word");
        defer(false);
        assert!(images.get(&later).is_none() && building());
        let mut world = World::new();
        world.insert_resource(images);
        let mut land = IntoSystem::into_system(land);
        land.initialize(&mut world);
        while building() {
            land.run((), &mut world).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let images = world.resource::<Assets<Image>>();
        assert_eq!(images.get(&later).expect("landed").data, images.get(&now).unwrap().data);
    }

    /// P57: the upscale that works each run of output pixels over the same four disc pixels out once gives the same
    /// pixels as working every one out from scratch, over clear, flat, soft and hard-edged patches.
    #[test]
    fn upscale_matches_the_plain_one() {
        let (w, h) = (37, 23);
        let mut seed = 7u32;
        let mut next = || {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            (seed >> 16) as f32 / 65536.0
        };
        let pm: Vec<[f32; 4]> = (0..w * h)
            .map(|i| match (i % w / 9, i / w / 6) {
                (0, _) => [0.0; 4],
                (1, _) => [0.25, 0.5, 0.125, 0.5],
                (2, y) if y % 2 == 0 => [0.9, 0.1, 0.1, 1.0],
                _ => {
                    let a = if next() < 0.3 { 0.0 } else { next() };
                    [next() * a, next() * a, next() * a, a]
                }
            })
            .collect();
        let (ow, oh) = (w as u32 * K, h as u32 * K);
        let [mut a, mut b] = [(); 2].map(|_| Pixmap::new(ow, oh).unwrap());
        upscale(&pm, w, h, &mut a, 0..oh as usize);
        // in bands, as the steps do
        for r in (0..oh as usize).step_by(13) {
            plain_upscale(&pm, w, h, &mut b, r..(r + 13).min(oh as usize));
        }
        assert!(a.data() == b.data());
    }

    #[test]
    fn wall_grid_drops_wobble() {
        let (paper, dark, faint) = ([200, 200, 200, 255], [100, 100, 100, 255], [150, 150, 150, 255]);
        let mut t = vec![paper; 16 * 16];
        (0..16).for_each(|i| (t[i], t[i * 16], t[8 * 16 + i], t[i * 16 + 8]) = (dark, dark, faint, faint));
        t[3 * 16 + 9] = faint; // wobble
        let out = grid(&t.concat(), 16, 16);
        let at = |x: usize, y: usize| -> [u8; 4] { out[(y * 16 * K as usize + x) * 4..][..4].try_into().unwrap() };
        let k = K as usize;
        assert_eq!((at(0, 5 * k), at(5 * k, 0), at(0, 0)), (dark, dark, dark));
        assert_eq!((at(8 * k, 5 * k), at(5 * k, 8 * k), at(8 * k, 0)), (faint, faint, dark));
        assert_eq!((at(9 * k, 3 * k), at(3 * k, 3 * k)), (paper, paper));
    }

    /// Our pill and box slices tile their box exactly, the pill's ends round (h / 2 wide), the box's corners `c`.
    #[test]
    fn pill_and_box_tile() {
        let q = pill_quads(2, [10.0, 20.0, 100.0, 30.0]);
        assert_eq!(q.map(|(_, d)| d), [[10.0, 20.0, 15.0, 30.0], [25.0, 20.0, 70.0, 30.0], [95.0, 20.0, 15.0, 30.0]]);
        let b = box_quads(1, [0.0, 0.0, 200.0, 100.0], 12.0);
        let area: f32 = b.iter().map(|(_, d)| d[2] * d[3]).sum();
        assert_eq!(area, 200.0 * 100.0);
        assert_eq!(b[8].1, [188.0, 88.0, 12.0, 12.0]);
        assert!(b.iter().all(|(s, _)| s[1] >= BOX_Y + 64.0 && s[1] + s[3] <= BOX_Y + 128.0));
    }

    /// A white word with a black ring on a transparent sheet comes back white-on-black, in its box, `K`× bigger.
    #[test]
    fn label_round_trip() {
        let (w, h) = (64u32, 32u32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 8..24 {
            for x in 8..56 {
                let edge = !(10..22).contains(&y) || !(10..54).contains(&x);
                rgba[((y * w + x) * 4) as usize..][..4].copy_from_slice(&if edge { [0, 0, 0, 255] } else { [255, 255, 255, 255] });
            }
        }
        let out = rebuild(w as usize, h as usize, &rgba, &[], &[(8, 8, 48, 16, "Ok")], HEAVY, false);
        assert_eq!((out.width(), out.height()), (w * K, h * K));
        let px = |x: u32, y: u32| out.pixel(x, y).unwrap().demultiply();
        assert_eq!(px(2, 2).alpha(), 0, "outside the box: cleared");
        let inked: Vec<_> = (0..out.height()).flat_map(|y| (0..out.width()).map(move |x| (x, y))).filter(|&(x, y)| px(x, y).alpha() > 128).collect();
        let (minx, maxx) = (inked.iter().map(|p| p.0).min().unwrap(), inked.iter().map(|p| p.0).max().unwrap());
        assert!(minx >= 8 * K - 2 && maxx <= 56 * K + 2, "fits its box: {minx}..{maxx}");
        assert!(inked.iter().any(|&(x, y)| px(x, y).red() > 240), "white fill");
        assert!(inked.iter().any(|&(x, y)| px(x, y).red() < 20), "black ring");
    }

    /// B141a: a stamp inks only inside its cell; the digit cells don't overlap; the meter icons ink their rects.
    #[test]
    fn stamps_and_icons_ink_their_cells() {
        let font = ab_glyph::FontRef::try_from_slice(HEAVY).unwrap();
        let mut out = Pixmap::new(128 * K, 64 * K).unwrap();
        stamp(&mut out, &font, [20.0, 10.0, 64.0, 41.0], "VS", &VS);
        let ink = |out: &Pixmap, [x, y, w, h]: [f32; 4]| {
            let k = K as f32;
            (0..out.height()).flat_map(|py| (0..out.width()).map(move |px| (px, py))).filter(|&(px, py)| out.pixel(px, py).unwrap().alpha() > 0).all(|(px, py)| {
                let (px, py) = (px as f32 / k, py as f32 / k);
                px >= x - 0.5 && px <= x + w + 0.5 && py >= y - 0.5 && py <= y + h + 0.5
            })
        };
        assert!(ink(&out, [20.0, 10.0, 64.0, 41.0]), "VS stays in its cell");
        let cells: Vec<_> = DIGITS_NOW.iter().map(|s| s.0).collect();
        for (i, a) in cells.iter().enumerate() {
            for b in &cells[i + 1..] {
                assert!(a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0, "{a:?} overlaps {b:?}");
            }
        }
        let mut sheet = Pixmap::new(64 * K, (ICON_Y as u32 + 72) * K).unwrap();
        meter_icons(&mut sheet);
        if let Ok(dir) = std::env::var("HST_MENU_ART") {
            image::save_buffer(format!("{dir}/pill_icons.png"), sheet.data(), sheet.width(), sheet.height(), image::ColorType::Rgba8).unwrap();
        }
        for r in [BALL, BALL_OFF, BOUNCE, SPEED] {
            let k = K as f32;
            let lit = (r[1] * k) as u32..((r[1] + r[3]) * k) as u32;
            assert!(lit.flat_map(|y| ((r[0] * k) as u32..((r[0] + r[2]) * k) as u32).map(move |x| (x, y))).any(|(x, y)| sheet.pixel(x, y).unwrap().alpha() > 128), "{r:?} inked");
        }
        assert!(ink(&sheet, [0.0, ICON_Y, 48.0, 72.0]), "icons stay in their rows");
    }
}
