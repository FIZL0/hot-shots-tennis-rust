//! B136: the menus' remastered art (ours, by the user's request). A menu sheet named in [`SHEETS`] is rebuilt at load
//! time at [`K`]× its size, in the same layout, so every screen's pixel rects still pick the same pieces
//! (`textures::ours` scales them): each label listed for it is erased from the disc picture and set again as vector
//! text in M PLUS Rounded 1c (OFL, `assets/fonts/`), filled with the label's own colours (sampled per row from the
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

static HEAVY: &[u8] = include_bytes!("../../assets/fonts/MPLUSRounded1c-ExtraBold.ttf");
static BOLD: &[u8] = include_bytes!("../../assets/fonts/MPLUSRounded1c-Bold.ttf");

/// A label: its ink box on the disc sheet (x, y, w, h) and its words.
type Label<'a> = (u16, u16, u16, u16, &'a str);

struct Sheet {
    name: &'static str,
    labels: &'static [Label<'static>],
    /// A screen's title bar: these words where the disc's title stands on its stripes (box found at run time), with a
    /// drop shadow.
    title: Option<&'static str>,
}

const fn sheet(name: &'static str, labels: &'static [Label<'static>]) -> Sheet {
    Sheet { name, labels, title: None }
}

const fn title(name: &'static str, words: &'static str) -> Sheet {
    Sheet { name, labels: &[], title: Some(words) }
}

const SHEETS: &[Sheet] = &[
    sheet("option_00", OPTION_00),
    sheet("word", &[]),
    sheet("characterselect2_00", CS0),
    sheet("characterselect2_02", NAMES),
    sheet("characterselect2_04", ATTRS),
    sheet("keyassign_characterselect2", KEYS),
    sheet("confirmation_01", COURTS),
    sheet("confirmation_03", CONFIRM),
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
    (1, 67, 42, 15, "sets"),
    (1, 82, 58, 24, "games"),
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
pub(crate) fn image(images: &mut Assets<Image>, data: &[u8], name: &str) -> Handle<Image> {
    let name = name.trim_start_matches('/').trim_end_matches(".tm2").to_ascii_lowercase();
    let sheet = SHEETS.iter().find(|s| s.name == name).filter(|_| !OFF.load(Ordering::Relaxed));
    let Some(sheet) = sheet else { return super::panel::image(images, data) };
    let pic = tim2::decode_alpha8(data).expect("TIM2").remove(0);
    let (w, h) = (pic.width as usize, pic.height as usize);
    let mut labels = sheet.labels.to_vec();
    let mut shadow = false;
    if let Some(words) = sheet.title {
        let (x, y, lw, lh) = title_box(&pic.rgba, w, h);
        labels.push((x, y, lw, lh, words));
        shadow = true;
    }
    let chars: Vec<(u16, u16, u16, u16, String)> = if name == "word" { glyph_boxes(&pic.rgba, w) } else { vec![] };
    labels.extend(chars.iter().map(|(x, y, lw, lh, c)| (*x, *y, *lw, *lh, c.as_str())));
    let out = rebuild(w, h, &pic.rgba, &labels, if name == "word" { BOLD } else { HEAVY }, shadow);
    if let Ok(dir) = std::env::var("HST_MENU_ART") {
        let _ = image::save_buffer(format!("{dir}/{name}.png"), out.data(), out.width(), out.height(), image::ColorType::Rgba8);
    }
    let (w, h) = (out.width(), out.height());
    let img = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, unpremultiply(out), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    let handle = images.add(img);
    crate::textures::ours(handle.id(), Vec2::splat(K as f32));
    handle
}

fn unpremultiply(p: Pixmap) -> Vec<u8> {
    p.pixels().iter().map(|c| c.demultiply()).flat_map(|c| [c.red(), c.green(), c.blue(), c.alpha()]).collect()
}

/// The sheet at `K`×: labels erased and set again, the rest snapped sharp.
fn rebuild(w: usize, h: usize, rgba8: &[u8], labels: &[Label], font: &[u8], shadow: bool) -> Pixmap {
    let mut rgba: Vec<[f32; 4]> = rgba8.chunks(4).map(|c| [c[0], c[1], c[2], c[3]].map(|v| v as f32 / 255.0)).collect();
    let styles: Vec<Style> = labels.iter().map(|l| Style::sample(&rgba, w, h, l)).collect();
    for &(x, y, lw, lh, t) in labels {
        // a title's old shadow stands 2 px down and right of its words
        let pad = if shadow { 3 } else { 0 };
        erase(&mut rgba, w, h, &(x, y, (lw + pad).min(w as u16 - x), (lh + pad).min(h as u16 - y), t));
    }
    let mut out = upscale(&rgba, w, h);
    let font = ab_glyph::FontRef::try_from_slice(font).expect("menu font");
    for (l, s) in labels.iter().zip(&styles) {
        label(&mut out, &font, l, s, shadow);
    }
    out
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

/// `word.tm2`'s glyphs: each cell's ink box and its letter (`main_menu::FONT`'s grid, 32 px cells from row 1).
fn glyph_boxes(rgba: &[u8], w: usize) -> Vec<(u16, u16, u16, u16, String)> {
    let mut out = vec![];
    for (row, chars) in super::main_menu::FONT.iter().enumerate() {
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
fn upscale(px: &[[f32; 4]], w: usize, h: usize) -> Pixmap {
    let (ow, oh) = (w * K as usize, h * K as usize);
    let mut out = Pixmap::new(ow as u32, oh as u32).expect("sheet size");
    // premultiplied, so a colour doesn't bleed out of a transparent neighbour
    let pm: Vec<[f32; 4]> = px.iter().map(|c| [c[0] * c[3], c[1] * c[3], c[2] * c[3], c[3]]).collect();
    let at = |x: isize, y: isize| pm[(y.clamp(0, h as isize - 1) as usize) * w + x.clamp(0, w as isize - 1) as usize];
    let data = out.pixels_mut();
    for oy in 0..oh {
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
    out
}

/// `s` as one path in font units (y down), and its advance.
fn text_path(font: &ab_glyph::FontRef, s: &str) -> Option<tiny_skia::Path> {
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
        let out = rebuild(w as usize, h as usize, &rgba, &[(8, 8, 48, 16, "Ok")], HEAVY, false);
        assert_eq!((out.width(), out.height()), (w * K, h * K));
        let px = |x: u32, y: u32| out.pixel(x, y).unwrap().demultiply();
        assert_eq!(px(2, 2).alpha(), 0, "outside the box: cleared");
        let inked: Vec<_> = (0..out.height()).flat_map(|y| (0..out.width()).map(move |x| (x, y))).filter(|&(x, y)| px(x, y).alpha() > 128).collect();
        let (minx, maxx) = (inked.iter().map(|p| p.0).min().unwrap(), inked.iter().map(|p| p.0).max().unwrap());
        assert!(minx >= 8 * K - 2 && maxx <= 56 * K + 2, "fits its box: {minx}..{maxx}");
        assert!(inked.iter().any(|&(x, y)| px(x, y).red() > 240), "white fill");
        assert!(inked.iter().any(|&(x, y)| px(x, y).red() < 20), "black ring");
    }
}
