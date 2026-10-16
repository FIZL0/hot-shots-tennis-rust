//! M5b: Installed → Texture packs: the installed texture packs (`mods_textures.rs`) as a list in their order, the top
//! one winning where two replace the same texture. Each row has its place, name, author, texture count and size, an
//! On / Off switch and ▲ / ▼ (a tap or a click on them; a pad's ✕ switches, △ moves up, □ moves down, Select removes,
//! as the chosen pack's panel and the legend say). A switch or a move is saved at once (settings.txt's
//! `texture_packs` / `texture_packs_off`) and, like an install, takes hold on Reload. The list keeps the grid's frame
//! (search, Show tabs, the footer's words): one tile a line (`grid::lay_of`), a row each.

use std::path::{Path, PathBuf};

use super::super::look;
use super::super::pointer::{Act, OK};
use super::super::roster::{PX, PY, ellipsis, rounded};
use super::super::{Draw, Effect, Menu, Press, WHITE};
use super::{Entry, Kind, Page, View, bare};
use crate::mods_textures;

/// A row's height (mockup px).
pub(super) const ROW: f32 = 46.0;

/// One installed texture pack as the list shows it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Row {
    pub dir: PathBuf,
    /// Its name in the saved order (`mods_textures::folder`).
    pub folder: String,
    pub name: String,
    pub author: String,
    pub count: usize,
    pub bytes: u64,
    pub on: bool,
}

/// The packs under `root` in the saved order, the ones switched off in their places.
pub(super) fn read(root: &Path) -> Vec<Row> {
    let all = mods_textures::ordered(root, mods_textures::list(root), &mods_textures::order());
    all.into_iter()
        .map(|(p, on)| Row {
            folder: mods_textures::folder(root, &p.dir),
            count: p.textures.len(),
            bytes: mods_textures::bytes(&p),
            dir: p.dir,
            name: p.name,
            author: p.author,
            on,
        })
        .collect()
}

/// "1 texture", "320 textures".
pub(super) fn count(n: usize) -> String {
    format!("{n} texture{}", if n == 1 { "" } else { "s" })
}

/// A pack's size: KB under a megabyte, else MB to a tenth.
pub(super) fn size(n: u64) -> String {
    if n < 1 << 20 { format!("{} KB", n.div_ceil(1 << 10)) } else { format!("{:.1} MB", n as f64 / (1 << 20) as f64) }
}

/// "1st", "2nd", "3rd", "11th", "22nd".
fn nth(n: usize) -> String {
    let s = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

impl View {
    /// The Texture packs tab of Installed: the packs' list.
    pub(super) fn packs_page(&self) -> bool {
        self.page == Page::Installed && self.kind == Kind::Textures
    }

    /// The list's row of tile `e`.
    pub(super) fn pack_of(&self, e: &Entry) -> Option<usize> {
        let dir = bare(self.dir_of(e)?);
        self.packs.iter().position(|r| bare(&r.dir) == dir)
    }

    /// The list's row under the cursor.
    pub(super) fn pack_at(&self, sel: usize) -> Option<&Row> {
        self.packs.get(self.pack_of(self.hovered(sel)?)?)
    }

    /// Where row `i` stands, for the panel and the status: "On - 1st of 3", "Off".
    pub(super) fn pack_place(&self, i: usize) -> String {
        let Some(r) = self.packs.get(i) else { return String::new() };
        if !r.on {
            return "Off: not used".into();
        }
        let at = self.packs[..i].iter().filter(|r| r.on).count() + 1;
        let of = self.packs.iter().filter(|r| r.on).count();
        if of == 1 { "On: the only pack in use".into() } else { format!("On: {} of {of} in use", nth(at)) }
    }

    /// The saved order is the list's.
    fn save_packs(&self) {
        let rows: Vec<(String, bool)> = self.packs.iter().map(|r| (r.folder.clone(), r.on)).collect();
        mods_textures::set_order(mods_textures::order_of(&rows));
    }
}

impl Menu {
    /// A press on a pack's row: ✕ switches it on or off, △ moves it up, □ down, Select removes it (asks first).
    pub(super) fn pack_press(&mut self, k: Press, out: &mut Vec<Effect>) {
        use Effect::Sound;
        let Some(i) = self.add.hovered(self.sel).and_then(|e| self.add.pack_of(e)) else { return };
        if k.hand {
            return self.installed_step(self.sel, out);
        }
        let v = &mut self.add;
        if k.ok {
            v.packs[i].on = !v.packs[i].on;
            let r = &v.packs[i];
            v.status = if r.on {
                format!("{} is on: {}. Reload to see it.", r.name, v.pack_place(i).trim_start_matches("On: "))
            } else {
                format!("{} is off: its textures come from the packs under it or the disc. Reload to see it.", r.name)
            };
        } else if k.card || k.page {
            let j = if k.card { i.checked_sub(1) } else { Some(i + 1).filter(|&j| j < v.packs.len()) };
            let Some(j) = j else {
                v.status = format!("{} is {} already.", v.packs[i].name, if k.card { "at the top" } else { "at the bottom" });
                return out.push(Sound(1));
            };
            v.packs.swap(i, j);
            let dir = v.packs[j].dir.clone();
            // the cursor goes with it
            if let Some(t) = v.tiles().iter().position(|e| v.dir_of(e).map(bare) == Some(bare(&dir))) {
                self.sel = v.acts() + t;
            }
            let r = &v.packs[j];
            v.status = match (r.on, j) {
                (true, 0) => format!("{} is on top: it wins over every other pack. Reload to see it.", r.name),
                (true, _) => format!("{} moved {}: {}. Reload to see it.", r.name, if k.card { "up" } else { "down" }, v.pack_place(j).trim_start_matches("On: ")),
                (false, _) => format!("{} moved {} (it's off).", r.name, if k.card { "up" } else { "down" }),
            };
        } else {
            return;
        }
        v.save_packs();
        v.changed();
        out.extend([Sound(0), Effect::SaveSettings]);
    }
}

/// An up (`up`) or down chevron centred on `[cx, cy]`.
fn chevron(d: &mut Draw, [cx, cy]: [f32; 2], up: bool, rgb: [f32; 3], alpha: f32) {
    let t = std::f32::consts::FRAC_PI_4 * if up { 1.0 } else { -1.0 };
    super::grid::bar(d, [cx - 3.0 * PX, cy], 8.0 * PX, 2.2 * PY, -t, rgb, alpha);
    super::grid::bar(d, [cx + 3.0 * PX, cy], 8.0 * PX, 2.2 * PY, t, rgb, alpha);
}

/// Row `i` of the list (`slot` its cursor slot) in `[x, y, w, h]`: a tile (the cursor's lit in the kind's accent
/// `a`), its place, its name over "by author - textures - size" (an off one's grey), then the On / Off switch and ▲ ▼
/// at its right, each a tap's press.
pub(super) fn row(d: &mut Draw, v: &View, a: [f32; 3], rect @ [x, y, w, h]: [f32; 4], on: bool, i: usize, slot: usize) {
    let Some(r) = v.packs.get(i) else { return };
    look::tile(d, rect, a, on);
    d.hit(rect, Act::Row(slot, Press::default()));
    let (ink, soft) = (look::c(look::INK), look::c(look::MUTED));
    let inset = if on { 7.0 } else { 5.0 };
    // the buttons, from the right: ▼, ▲, the switch
    let bh = (h - 2.0 * (inset + 4.0) * PY).min(28.0 * PY);
    let by = y + (h - bh) / 2.0;
    let bw = bh * PX / PY;
    let mut bx = x + w - (inset + 4.0) * PX;
    let n = v.packs.len();
    for (up, can) in [(false, i + 1 < n), (true, i > 0)] {
        bx -= bw;
        let b = [bx, by, bw, bh];
        look::outlined_pill(d, b, if can { look::c(look::PAPER) } else { look::c(look::TILE) }, 1.5);
        chevron(d, [bx + bw / 2.0, by + bh / 2.0 + if up { -1.0 } else { 1.0 } * PY], up, ink, if can { 128.0 } else { 40.0 });
        d.hit(b, Act::Row(slot, Press { card: up, page: !up, ..Press::default() }));
        bx -= 6.0 * PX;
    }
    let sw = 52.0 * PX;
    bx -= sw;
    let b = [bx, by, sw, bh];
    look::outlined_pill(d, b, look::c(if r.on { look::MINT } else { look::GREY }), 1.5);
    look::pill_words(d, b, if r.on { "On" } else { "Off" }, 13.0 * PY, if r.on { ink } else { WHITE });
    d.hit(b, Act::Row(slot, OK));
    // the place, the name and its line
    let tx0 = x + (inset + 6.0) * PX;
    let ps = 15.0 * PY;
    let place = (i + 1).to_string();
    let pw = d.width(&format!("{n}"), ps).max(d.width("9", ps)) + 10.0 * PX;
    d.text(&place, tx0, y + (h - ps) / 2.0, ps, soft);
    let tx = tx0 + pw;
    let room = (bx - 10.0 * PX - tx).max(0.0);
    let (ns, ls) = (14.5 * PY, 12.0 * PY);
    let ty = y + (h - ns - ls - 3.0 * PY) / 2.0;
    let name = ellipsis(d, &r.name, ns, room);
    let fill = if r.on { ink } else { soft };
    d.text(&name, tx, ty, ns, fill);
    d.text(&name, tx + 0.6 * PX, ty, ns, fill);
    let by_line = if r.author.is_empty() { String::new() } else { format!("by {} - ", r.author) };
    let line = ellipsis(d, &format!("{by_line}{} - {}", count(r.count), size(r.bytes)), ls, room);
    d.text(&line, tx, ty + ns + 3.0 * PY, ls, soft);
    if !r.on {
        // (off: the tile dimmed but its switch)
        rounded(d, [x + 2.0 * PX, y + 2.0 * PY, (b[0] - 4.0 * PX - x).max(0.0), h - 4.0 * PY], 10.0, WHITE, 30.0);
    }
}
