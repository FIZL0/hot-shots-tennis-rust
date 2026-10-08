//! The match's weather moves on a game at a time: the original steps its weather and wind counters after each
//! game and applies them when the next point is set up. Rain makes running slower to pick up (`Stats::new`'s
//! agility ×1.5).

use bevy::prelude::*;

use super::{Game, Phase};
use crate::weather::Weather;

pub fn plugin(app: &mut App) {
    app.add_systems(FixedUpdate, step);
}

/// Each player's clear-weather agility, taken at the first tick.
fn step(mut g: ResMut<Game>, w: Option<ResMut<Weather>>, mut clear: Local<Vec<i32>>) {
    let Some(mut w) = w else { return };
    if g.phase == Phase::Serve && w.game != g.score.games_played as usize {
        w.game = g.score.games_played as usize;
    }
    if clear.len() != g.players.len() {
        *clear = g.players.iter().map(|p| p.stats.agility).collect();
    }
    let rain = hst_sim::weather::rain(w.today().weather);
    for (p, a) in g.players.iter_mut().zip(clear.iter()) {
        p.stats.agility = if rain { a * 150 / 100 } else { *a };
    }
}
