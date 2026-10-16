//! P61: what the match asks of the front-end above it (the pause screen `menu.rs`, the select's preview cache in
//! `main_menu/`), which the match's crate can't name: the app inserts [`FrontEnd`] with the match (`main.rs`
//! `add_match`). A bare match (most tests) has none: never paused, the disc's own faces.

use bevy::prelude::*;
use crate::mods::Mod;

#[derive(Resource, Clone, Copy)]
pub struct FrontEnd {
    /// The pause screen is up.
    pub paused: fn(&World) -> bool,
    /// A peer opened or closed the pause screen, or the app came back from the background: here too.
    pub set_paused: fn(&mut World, bool),
    /// B133: player face icon for the match panel from the select's preview of mod `m`'s costume `k`, else disc
    /// character `c`'s (`iso`, `m`, `c`, `k`); None until that's baked.
    pub face: fn(&str, Option<&Mod>, usize, usize) -> Option<Image>,
}

/// The pause screen is up.
pub fn paused(w: &World) -> bool {
    w.get_resource::<FrontEnd>().is_some_and(|f| (f.paused)(w))
}

/// Opens or closes the pause screen (see [`FrontEnd::set_paused`]).
pub fn set_paused(w: &mut World, open: bool) {
    if let Some(f) = w.get_resource::<FrontEnd>().copied() {
        (f.set_paused)(w, open);
    }
}

pub fn plugin(app: &mut App) {
    // every sound from here on is the match's until the mode is torn down (`mode.rs`), held while paused
    app.add_systems(PreStartup, |s: Option<Res<crate::audio::Sound>>| s.iter().for_each(|s| s.begin_match()));
}
