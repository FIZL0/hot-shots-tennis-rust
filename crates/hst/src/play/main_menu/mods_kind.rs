//! B373: the Mods page's kind tabs (spec `plan/B372.md` § Kinds); B383: the page's one tab row, in the title band
//! between the L2 / R2 buttons (keyboard 1 / 3) in the legend's art: Characters · Courts · Umpires · Texture packs,
//! each with its count (a kind with none hidden until it has some), the current one filled in its colours (B448); how the kind behaves online
//! is the footer's line. Every Show (Everything, Installed, My uploads, …) and Upload lists only the chosen kind (`modstore::Meta::kind`; an installed mod's from its `mod.json`). The store
//! carries characters, courts and umpires (M8f9); the Texture packs tab shows what is coming and from which task (M5)
//! until it lands.

use super::look;
use super::roster::{self, PX, PY, Seat};
use super::{Draw, pointer};
use crate::modstore::Kind;

/// The tabs, in L2 / R2 order.
pub(super) const KINDS: [Kind; 4] = [Kind::Character, Kind::Court, Kind::Umpire, Kind::Textures];

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
        Kind::Character | Kind::Other => "Online: players without this mod see a stand-in character",
        Kind::Court => "Online: players without it play the disc court it's built on",
        Kind::Umpire => "Online: a missing call falls back to the disc umpire",
        Kind::Textures => "Online: only you see a texture pack; the lobby doesn't share it",
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

/// B383: the tabs the row shows: the kinds with mods (`total`, by [`KINDS`]: on the store or installed; one the
/// store doesn't carry yet has none), and the chosen one.
pub(super) fn shown(at: Kind, total: [usize; 4]) -> Vec<Kind> {
    KINDS.iter().zip(total).filter(|(k, n)| **k == at || *n > 0 && coming(**k).is_none()).map(|(k, _)| *k).collect()
}

/// The tab after `at` among `shown` (L2 back, R2 on), round.
pub(super) fn step(at: Kind, back: bool, shown: &[Kind]) -> Kind {
    let n = shown.len();
    let Some(i) = shown.iter().position(|k| *k == at) else { return shown.first().copied().unwrap_or(at) };
    shown[if back { (i + n - 1) % n } else { (i + 1) % n }]
}

/// B448: each kind's colour set, a seat's of the character select (`roster::seat`): Characters red, Courts green,
/// Umpires blue, Texture packs gold.
pub(super) fn seat_of(k: Kind) -> usize {
    match k {
        Kind::Character | Kind::Other => 0,
        Kind::Court => 3,
        Kind::Umpire => 2,
        Kind::Textures => 1,
    }
}

/// B383: the page's one tab row, in the top bar from `x` (`top` its top, `room` wide): the L2 / R2 buttons in the
/// legend's art (the device last used: a pad's, or the keyboard's 1 / 3 keys) round the `shown` kinds' tabs, each
/// with its count (`counts`, by [`KINDS`]; a tap on one is `Act::Kind(i)`). B448: each tab a ring pill in its kind's
/// colours (`seat_of`), the current one filled. The buttons go while there's only one kind to show. The row shrinks
/// as one where it doesn't fit.
pub(super) fn row(d: &mut Draw, [x, top, room]: [f32; 3], at: Kind, counts: [usize; 4], shown: &[Kind]) {
    let idx: Vec<usize> = shown.iter().filter_map(|k| KINDS.iter().position(|q| q == k)).collect();
    let words: Vec<String> = idx.iter().map(|&i| format!("{} {}", name(KINDS[i]), counts[i])).collect();
    let seats: Vec<Seat> = idx.iter().map(|&i| roster::seat(seat_of(KINDS[i]))).collect();
    let on = idx.iter().position(|&i| KINDS[i] == at);
    look::tabs(d, [x, top, room], &words, &seats, on, Some((0, 1)), |j| pointer::Act::Kind(idx[j]));
}

/// The empty state of a kind the store doesn't carry yet, in the page's column `[x, y, w, h]`.
pub(super) fn empty(d: &mut Draw, k: Kind, [x, y, w, h]: [f32; 4]) {
    let Some(say) = coming(k) else { return };
    let s = roster::seat(seat_of(k));
    d.label(name(k), x + w / 2.0, y + h * 0.3, 26.0 * PY, w - 40.0 * PX, s.ring);
    let size = 16.0 * PY;
    let lw = (w - 60.0 * PX).min(420.0 * PX);
    look::para(d, say, [x + (w - lw) / 2.0, y + h * 0.3 + 42.0 * PY], lw, size, 22.0 * PY, 6, s.soft);
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
            k = step(k, false, &KINDS);
            seen.push(k);
        }
        assert_eq!(seen, [Kind::Court, Kind::Umpire, Kind::Textures, Kind::Character]);
        assert_eq!(step(Kind::Character, true, &KINDS), Kind::Textures);
        assert_eq!(step(Kind::Other, false, &KINDS), Kind::Character);
        // B383: a kind with no mods is hidden (the chosen one stays); texture packs wait for M5 whatever the count
        assert_eq!(shown(Kind::Character, [3, 0, 2, 5]), [Kind::Character, Kind::Umpire]);
        assert_eq!(shown(Kind::Court, [0, 0, 0, 0]), [Kind::Court]);
        assert_eq!(step(Kind::Umpire, false, &[Kind::Character, Kind::Umpire]), Kind::Character);
        assert_eq!(step(Kind::Court, true, &[Kind::Court]), Kind::Court);
        assert!([Kind::Character, Kind::Court, Kind::Umpire].iter().all(|k| coming(*k).is_none()));
        assert!(coming(Kind::Textures).unwrap().contains("M5") && coming(Kind::Textures).unwrap().contains("mods/texture-replacements/"));
        assert!(KINDS.iter().all(|k| online(*k).starts_with("Online: ")));
    }
}
