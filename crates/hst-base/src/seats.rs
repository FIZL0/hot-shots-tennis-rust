//! What drives a player seat and which gamepads are read at all (`play::controls` re-exports both): here, below the
//! match, for `Args` and the chat (P60).

use bevy::prelude::*;

/// What drives one player seat (`--pads`, from the main menu's controller assignment): the keyboard or a gamepad (an
/// index into the sorted pad list).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Seat {
    Keys,
    Pad(usize),
}

impl Seat {
    /// `k` the keyboard, a number a gamepad, anything else (`-`) the computer.
    pub fn parse(s: &str) -> Option<Seat> {
        match s.trim() {
            "k" => Some(Seat::Keys),
            n => n.parse().ok().map(Seat::Pad),
        }
    }

    /// `--pads` for seats 1P..4P (`-` the computer).
    pub fn flag(seats: &[Option<Seat>]) -> String {
        let one = |s: &Option<Seat>| match s {
            Some(Seat::Keys) => "k".to_string(),
            Some(Seat::Pad(n)) => n.to_string(),
            None => "-".into(),
        };
        seats.iter().map(one).collect::<Vec<_>>().join(",")
    }
}

/// Whether the app reads this pad. Under Steam Input, Steam lists the physical pads it stands in for in
/// `SDL_GAMECONTROLLER_IGNORE_DEVICES` (`0xVID/0xPID,…`); those are skipped so only Steam's virtual pad, with the
/// player's Steam/Big Picture layout, drives the game. Outside Steam Input, Steam's background virtual pads (Valve
/// 0x28de/0x11ff) mirror real ones and are skipped instead.
pub fn readable(g: &Gamepad) -> bool {
    static IGNORED: std::sync::OnceLock<Option<Vec<(u16, u16)>>> = std::sync::OnceLock::new();
    let ignored = IGNORED.get_or_init(|| std::env::var("SDL_GAMECONTROLLER_IGNORE_DEVICES").ok().map(|v| ignore_list(&v)));
    let id = (g.vendor_id().unwrap_or(0), g.product_id().unwrap_or(0));
    match ignored {
        Some(list) => !list.contains(&id),
        None => id != (0x28de, 0x11ff),
    }
}

/// `SDL_GAMECONTROLLER_IGNORE_DEVICES`' `0xVID/0xPID` pairs; malformed entries are dropped.
fn ignore_list(v: &str) -> Vec<(u16, u16)> {
    let hex = |s: &str| u16::from_str_radix(s.trim().trim_start_matches("0x").trim_start_matches("0X"), 16).ok();
    v.split(',').filter_map(|e| e.split_once('/')).filter_map(|(a, b)| Some((hex(a)?, hex(b)?))).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn steam_ignore_list() {
        assert_eq!(super::ignore_list("0x045e/0x028e,0x054C/0x09cc,,junk,0x28de/"), [(0x045e, 0x028e), (0x054c, 0x09cc)]);
    }
}
