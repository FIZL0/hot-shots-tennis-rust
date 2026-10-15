//! B373: the Mods pages' kind tabs (spec `plan/B372.md` § Kinds). A row under the page tabs, between the L2 / R2
//! glyphs (keyboard 1 / 3, the keys over Q / E): Characters · Courts · Umpires · Texture packs, each with its count,
//! the current one gold, and on the right how the kind behaves online. Every page (Browse, Installed, Upload, My
//! uploads) lists only the chosen kind (`modstore::Meta::kind`; an installed mod's from its `mod.json`). The store
//! carries characters, courts and umpires (M8f9); the Texture packs tab shows what is coming and from which task (M5)
//! until it lands.

use super::{BROWN, CELL_FILL, Draw, GOLD, INK, WHITE, legend, pointer};
use crate::modstore::Kind;

/// The tabs, in L2 / R2 order.
pub(super) const KINDS: [Kind; 4] = [Kind::Character, Kind::Court, Kind::Umpire, Kind::Textures];

/// The row's height, under the page tabs and over the page's boxes.
pub(super) const ROW_H: f32 = 34.0;

pub(super) fn name(k: Kind) -> &'static str {
    match k {
        Kind::Character | Kind::Other => "Characters",
        Kind::Court => "Courts",
        Kind::Umpire => "Umpires",
        Kind::Textures => "Texture packs",
    }
}

/// How a kind behaves online (the note at the row's right).
pub(super) fn online(k: Kind) -> &'static str {
    match k {
        Kind::Character | Kind::Other => "Online: a player without the mod sees a stand-in",
        Kind::Court => "Online: a player without it plays its disc layout",
        Kind::Umpire => "Online: a missing call falls back to the disc umpire",
        Kind::Textures => "Online: looks only, per player, never synced to a lobby",
    }
}

/// What a kind's tab says while the store doesn't carry it yet (None: it does).
pub(super) fn coming(k: Kind) -> Option<&'static str> {
    match k {
        Kind::Character | Kind::Court | Kind::Umpire => None,
        Kind::Textures | Kind::Other => {
            Some("Texture packs arrive with M5: until then, install packs by hand in mods/texture-replacements/.")
        }
    }
}

/// The tab after `at` (L2 back, R2 on), round.
pub(super) fn step(at: Kind, back: bool) -> Kind {
    let i = KINDS.iter().position(|k| *k == at).unwrap_or(0);
    KINDS[if back { (i + KINDS.len() - 1) % KINDS.len() } else { (i + 1) % KINDS.len() }]
}

/// The row in `[x, y, w, h]`: the glyphs, the tabs with `counts` (by [`KINDS`]; a tap on one is `Act::Kind(i)`)
/// and the note. The tabs shrink to leave the note room, and the note shrinks to what's left.
pub(super) fn row(d: &mut Draw, [x, y, w, h]: [f32; 4], at: Kind, counts: [usize; 4]) {
    let caps = legend::kind_caps();
    let (size, ch) = (14.0, h - 8.0);
    let cap_w = |d: &Draw, s: &str| d.width(s, 12.0) + 12.0;
    let words: Vec<String> = KINDS.iter().zip(counts).map(|(k, n)| format!("{} {n}", name(*k))).collect();
    let ws: Vec<f32> = words.iter().map(|s| d.width(s, size) + 20.0).collect();
    let natural = cap_w(d, &caps[0]) + cap_w(d, &caps[1]) + ws.iter().sum::<f32>() + 6.0 * 5.0;
    let k = (w * 0.68 / natural).min(1.0);
    let mut cx = x;
    let cap = |d: &mut Draw, s: &str, cx: &mut f32| {
        let cw = cap_w(d, s) * k;
        d.pill_at([*cx, y + 4.0 + ch * 0.15, cw, ch * 0.7], [INK, [90.0; 3], [100.0; 3]], 128.0);
        d.label(s, *cx + cw / 2.0, y + 4.0 + ch * 0.15 + (ch * 0.7 - 12.0) / 2.0, 12.0, cw - 6.0, INK);
        *cx += cw + 6.0 * k;
    };
    cap(d, &caps[0], &mut cx);
    for (i, (s, tw)) in words.iter().zip(&ws).enumerate() {
        let on = KINDS[i] == at;
        let b = [cx, y + 4.0, tw * k, ch];
        d.pill_at(b, [INK, if on { INK } else { [90.0; 3] }, if on { GOLD } else { CELL_FILL }], 128.0);
        d.label(s, cx + tw * k / 2.0, y + 4.0 + (ch - size) / 2.0, size, (tw - 14.0) * k, if on { INK } else { BROWN });
        d.hit(b, pointer::Act::Kind(i));
        cx += (tw + 6.0) * k;
    }
    cap(d, &caps[1], &mut cx);
    let room = x + w - cx - 8.0;
    if room > 40.0 {
        // smaller rather than squeezed where it doesn't fit (a 4:3 window)
        let note = online(at);
        let size = (13.0 * room / d.width(note, 13.0)).min(13.0);
        let nw = d.width(note, size);
        d.outlined(|d| d.text(note, x + w - nw, y + 4.0 + (ch - size) / 2.0, size, WHITE));
    }
}

/// The empty state of a kind the store doesn't carry yet, in the page's list box `[x, y, w, h]`.
pub(super) fn empty(d: &mut Draw, k: Kind, [x, y, w, h]: [f32; 4]) {
    let Some(say) = coming(k) else { return };
    let size = 17.0;
    let title = name(k);
    d.label(title, x + w / 2.0, y + h * 0.32, 26.0, w - 40.0, BROWN);
    let mut lines: Vec<String> = Vec::new();
    for word in say.split_whitespace() {
        match lines.last_mut() {
            Some(l) if d.width(&format!("{l} {word}"), size) <= w - 60.0 => *l = format!("{l} {word}"),
            _ => lines.push(word.into()),
        }
    }
    for (i, l) in lines.iter().enumerate() {
        d.label(l, x + w / 2.0, y + h * 0.32 + 40.0 + 22.0 * i as f32, size, w - 40.0, super::options::MUTED);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L2 / R2 go round the four tabs; characters, courts and umpires are in the store (M8f9), texture packs name
    /// the task that brings them.
    #[test]
    fn tabs_go_round_and_say_what_is_coming() {
        let mut k = Kind::Character;
        let mut seen = Vec::new();
        for _ in 0..4 {
            k = step(k, false);
            seen.push(k);
        }
        assert_eq!(seen, [Kind::Court, Kind::Umpire, Kind::Textures, Kind::Character]);
        assert_eq!(step(Kind::Character, true), Kind::Textures);
        assert_eq!(step(Kind::Other, false), Kind::Court);
        assert!([Kind::Character, Kind::Court, Kind::Umpire].iter().all(|k| coming(*k).is_none()));
        assert!(coming(Kind::Textures).unwrap().contains("M5") && coming(Kind::Textures).unwrap().contains("mods/texture-replacements/"));
        assert!(KINDS.iter().all(|k| online(*k).starts_with("Online: ")));
    }
}
