//! The match's weather on court (`hst_sim::weather`): which game of the schedule is on, and what its weather
//! does to the fog, light, clear colour, ground shadow and background models. Clouds read it in `main::clouds`.
//! The wind's speed also sways the costumes (`noise`).
//! The ball's and NPCs' light scale: read from the court's sun-shade map in clear and cloudy weather, 1.0 in rain
//! (`shade`).

use std::collections::HashSet;
use std::sync::atomic::{AtomicU8, Ordering};

use bevy::prelude::*;
use hst_sim::weather::{self, Game, GAMES};

use crate::gs::{self, GsMaterial};

static NOW: AtomicU8 = AtomicU8::new(0);

/// The weather on court now (0 clear).
pub fn now() -> u8 {
    NOW.load(Ordering::Relaxed)
}

/// The match's schedule and the game it is on; `fixed` forces one weather (`HST_WEATHER`).
#[derive(Resource)]
pub struct Weather {
    pub schedule: [Game; GAMES],
    pub game: usize,
    pub fixed: Option<u8>,
}

impl Weather {
    pub fn today(&self) -> Game {
        let mut g = self.schedule[self.game % GAMES];
        if let Some(w) = self.fixed {
            g.weather = w;
        }
        g
    }
}

/// What `apply` needs to redo the court's look: the envir file, the drawn sky's colour, the per-weather look
/// rows and the court's materials and models by category.
#[derive(Resource)]
pub struct CourtLook {
    pub envir: Vec<u8>,
    pub season: usize,
    pub sky: Option<[f32; 3]>,
    pub looks: [[f32; 6]; 6],
    /// The players' light factors (`exe::Game::player_light`).
    pub players: [f32; 4],
    /// Background models (`_bg`, `_acc`, `_clo`) and the sky: their own flat fogs.
    pub bg: HashSet<AssetId<GsMaterial>>,
    pub skies: HashSet<AssetId<GsMaterial>>,
    /// The hole's ground (the shadow receiver).
    pub holes: HashSet<AssetId<GsMaterial>>,
    /// `_acc` and `_clo` models.
    pub acc: Vec<Entity>,
    pub clo: Vec<Entity>,
    pub last: Option<u8>,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, apply);
}

pub(crate) fn apply(
    mut commands: Commands,
    weather: Option<Res<Weather>>,
    look: Option<ResMut<CourtLook>>,
    sun: Option<Res<crate::shadow::Sun>>,
    mut materials: ResMut<Assets<GsMaterial>>,
    mut vis: Query<&mut Visibility>,
) {
    let (Some(weather), Some(mut look)) = (weather, look) else { return };
    let w = weather.today().weather;
    NOW.store(w, Ordering::Relaxed);
    if look.last == Some(w) {
        return;
    }
    look.last = Some(w);
    let l = look.looks.get(w as usize).map_or(weather::Look::CLEAR, |r| weather::look(w, *r));
    let fog = gs::court_fog(&look.envir, look.season, &l);
    let light = gs::court_light(&look.envir, look.season, &l).zip(sun.as_deref());
    let darken = sun.as_deref().map_or(0.0, |s| crate::shadow::darken_s(weather::shadow(s.strength, w)));
    for (id, m) in materials.iter_mut() {
        if let Some((main, bg, sky, colour)) = fog {
            m.uniform.fog = if look.skies.contains(&id) { sky } else if look.bg.contains(&id) { bg } else { main };
            m.uniform.fog_color = colour;
        }
        if let Some(((colour, ambient), sun)) = light {
            (m.uniform.light_dir, m.uniform.light_color, m.uniform.ambient) = (sun.light.extend(0.0), colour, ambient);
        }
        if look.holes.contains(&id) {
            m.uniform.shadow = darken;
        }
    }
    if let Some([r, g, b]) = look.sky.and_then(|c| gs::court_clear(&look.envir, look.season, c, &l)) {
        commands.insert_resource(ClearColor(Color::srgb_u8(r, g, b)));
    }
    let (acc, clo, _sun) = weather::shows(w);
    for (list, on) in [(&look.acc, acc), (&look.clo, clo)] {
        for e in list {
            if let Ok(mut v) = vis.get_mut(*e) {
                *v = if on { Visibility::Inherited } else { Visibility::Hidden };
            }
        }
    }
}
