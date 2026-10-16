//! M97 (user 2026-10-15; ours, menus aren't 1:1): main menu → Extras, a list on the left like the main screen's (only
//! Donate so far) and its panel on the right. Donate shows what building the remaster has taken (the user's prompts,
//! the tokens used, commits and sessions: `web/stats.json`, written by `tools/dev_stats.py` at release time, the same
//! file the site's Donate section reads) and opens the Ko-fi page. Laid out in home's columns, so it scales uniformly
//! at any aspect ratio.

use super::home::{self, FACT_H, FOOT, LIST_W, MUTED, OFF, ON, PAPER, PIC_H, PILL_H, PITCH, TOP, fit_text, para};
use super::{Draw, Effect, GOLD, INK, Menu, Press, Screen, Tex, WHITE, pointer};

/// The donate link (user 2026-10-15, plan/HUMAN.md; also `.github/FUNDING.yml` and the site's Donate section).
pub(super) const KOFI: &str = "https://ko-fi.com/fizlo";

/// Extras' rows, top to bottom.
pub(super) const ROWS: [&str; 1] = ["Donate"];

/// The counts in `web/stats.json`, built in (release.sh commits them before it builds).
#[derive(Debug, PartialEq)]
pub(super) struct Stats {
    pub prompts: u64,
    pub tokens: u64,
    pub commits: u64,
    pub sessions: u64,
    pub since: String,
    pub updated: String,
}

pub(super) fn stats() -> &'static Option<Stats> {
    static S: std::sync::OnceLock<Option<Stats>> = std::sync::OnceLock::new();
    S.get_or_init(|| parse(include_str!("../../../../../web/stats.json")))
}

fn parse(s: &str) -> Option<Stats> {
    let v: serde_json::Value = serde_json::from_str(s).ok()?;
    let n = |k: &str| v[k].as_u64();
    let s = |k: &str| v[k].as_str().unwrap_or_default().to_string();
    Some(Stats { prompts: n("prompts")?, tokens: n("tokens")?, commits: n("commits")?, sessions: n("sessions")?, since: s("since"), updated: s("updated") })
}

/// `x` short, as the site writes it: 7.0 billion, 37.3 million, 1,585.
pub(super) fn short(x: u64) -> String {
    match x {
        1_000_000_000.. => format!("{:.1} billion", x as f64 / 1e9),
        1_000_000.. => format!("{:.1} million", x as f64 / 1e6),
        _ => {
            let d = x.to_string();
            let mut out = String::new();
            for (i, c) in d.chars().enumerate() {
                if i > 0 && (d.len() - i).is_multiple_of(3) {
                    out.push(',');
                }
                out.push(c);
            }
            out
        }
    }
}

impl Menu {
    /// Extras' presses: ↑↓ move, ✕ does the row (Donate opens Ko-fi), ○ back to the main menu's Extras row.
    pub(super) fn extras_step(&mut self, k: Press, out: &mut Vec<Effect>) {
        if k.up || k.down {
            let n = ROWS.len();
            self.sel = if k.down { (self.sel + 1) % n } else { (self.sel + n - 1) % n };
            out.push(Effect::Sound(0));
        }
        if k.ok {
            out.push(Effect::Sound(2));
            // not from the tests (the legend's probe presses ✕ on every screen): no browser on the desktop
            if !cfg!(test) {
                crate::modstore::open(KOFI);
            }
        } else if k.back {
            self.sel = self.main_at(home::Row::Extras);
            self.screen = Screen::Main;
            out.push(Effect::Sound(1));
        }
    }
}

/// The Extras screen.
pub(super) fn layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    d.frame(10, 5);
    let (x, px, pw, _) = home::columns(m.grid.extra);
    let sel = m.sel.min(ROWS.len() - 1);
    let mut hand = [0.0; 2];
    for (i, name) in ROWS.iter().enumerate() {
        let on = i == sel;
        let rect = [x, TOP + PITCH * i as f32, LIST_W, PILL_H];
        d.pill_at(rect, [INK, WHITE, if on { ON } else { OFF }], 128.0);
        fit_text(d, name, x + 20.0, rect[1] + 7.0, 24.0, LIST_W - 56.0, if on { INK } else { [100.0; 3] });
        if on {
            d.text(">", x + LIST_W - 28.0, rect[1] + 7.0, 24.0, INK);
            hand = [rect[0] - 60.0, rect[1] + rect[3] / 2.0 - 16.0];
        }
        d.hit(rect, pointer::Act::Row(i, pointer::OK));
    }
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [hand[0] + hand_x, hand[1], 64.0, 32.0], WHITE, 128.0);
    donate(d, [px, TOP, pw, FOOT - TOP]);
}

/// Donate's panel in `[x, y, w, h]`: the counts as big tiles over a gold picture, the words, three facts.
fn donate(d: &mut Draw, [x, y, w, h]: [f32; 4]) {
    d.box3([x, y, w, h], 16.0, [Some(INK), Some(WHITE), Some(PAPER)], 118.0);
    let pic = [x + 3.0, y + 3.0, w - 6.0, PIC_H];
    d.box3(pic, 13.0, [None, None, Some([118.0, 104.0, 70.0])], 128.0);
    d.q(Tex::Solid, [0.0; 4], [x, y + 3.0 + PIC_H, w, 2.5], INK, 128.0);
    let stats = stats().as_ref();
    if let Some(s) = stats {
        let tiles = [(s.prompts, "PROMPTS"), (s.tokens, "TOKENS"), (s.commits, "COMMITS"), (s.sessions, "SESSIONS")];
        let (gap, th) = (8.0, 92.0);
        let tw = (pic[2] - 28.0 - 3.0 * gap) / 4.0;
        for (k, (n, label)) in tiles.into_iter().enumerate() {
            let r = [pic[0] + 14.0 + (tw + gap) * k as f32, pic[1] + 14.0, tw, th];
            d.box3(r, 10.0, [Some(INK), Some(WHITE), Some([92.0, 74.0, 34.0])], 128.0);
            // "7.0 billion": the number big, its unit under it
            let (num, unit) = short(n).split_once(' ').map_or((short(n), String::new()), |(a, b)| (a.into(), b.into()));
            d.outlined(|d| d.label(&num, r[0] + tw / 2.0, r[1] + 12.0, 30.0, tw - 14.0, WHITE));
            if !unit.is_empty() {
                d.label(&unit, r[0] + tw / 2.0, r[1] + 44.0, 13.0, tw - 14.0, WHITE);
            }
            d.label(label, r[0] + tw / 2.0, r[1] + th - 22.0, 12.0, tw - 14.0, GOLD);
        }
    }
    d.outlined(|d| d.text("Donate", x + 18.0, y + PIC_H - 34.0, 34.0, WHITE));
    let words = "OpenTennis is free, and always will be: no paid builds, no locked features, no early access. If you'd like to support my work, you can chip in on Ko-fi. Above: what building it has taken so far.";
    para(d, words, x + 18.0, y + PIC_H + 16.0, w - 36.0, 15.0, 5, INK);
    let mut facts = vec![("Ko-fi", "ko-fi.com/fizlo".to_string())];
    if let Some(s) = stats {
        facts.extend([("Since", s.since.clone()), ("Counted", s.updated.clone())]);
    }
    let n = facts.len() as f32;
    let tw = (w - 36.0 - 10.0 * (n - 1.0)) / n;
    for (i, (k, v)) in facts.iter().enumerate() {
        let tx = x + 18.0 + (tw + 10.0) * i as f32;
        let ty = y + h - 16.0 - FACT_H;
        d.box3([tx, ty, tw, FACT_H], 10.0, [Some(INK), Some(PAPER), Some(PAPER)], 128.0);
        fit_text(d, &k.to_uppercase(), tx + 10.0, ty + 6.0, 12.0, tw - 20.0, MUTED);
        fit_text(d, v, tx + 10.0, ty + 21.0, 17.0, tw - 20.0, INK);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The built-in stats parse (a bad web/stats.json would hide the tiles) and read as the site writes them.
    #[test]
    fn stats_parse_and_read_short() {
        let s = stats().as_ref().expect("web/stats.json");
        assert!(s.prompts > 0 && s.tokens > s.prompts && !s.since.is_empty());
        assert_eq!(short(6_976_022_331), "7.0 billion");
        assert_eq!(short(37_253_923), "37.3 million");
        assert_eq!((short(1585), short(999), short(123_456)), ("1,585".into(), "999".into(), "123,456".into()));
    }

    /// Main → Extras → ○ lands back on the main menu's Extras row.
    #[test]
    fn extras_and_back() {
        let mut m = Menu::default();
        m.sel = m.main_at(home::Row::Extras);
        m.step(super::super::Dev::Keys, Press { ok: true, ..Default::default() });
        assert_eq!((m.screen, m.sel), (Screen::Extras, 0));
        m.step(super::super::Dev::Keys, Press { back: true, ..Default::default() });
        assert_eq!((m.screen, m.sel), (Screen::Main, m.main_at(home::Row::Extras)));
    }
}
