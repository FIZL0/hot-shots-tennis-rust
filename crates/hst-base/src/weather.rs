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
#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct Weather {
    #[serde(with = "hst_sim::big")]
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
    pub sky: Option<[f32; 3]>,
    pub looks: [[f32; 6]; 6],
    /// The players' light factors (`exe::Game::player_light`).
    pub players: [f32; 4],
    /// Background models (`_bg`, `_acc`, `_clo`) and the sky: their own flat fogs.
    pub bg: HashSet<AssetId<GsMaterial>>,
    pub skies: HashSet<AssetId<GsMaterial>>,
    /// The hole's ground (the shadow receiver).
    pub holes: HashSet<AssetId<GsMaterial>>,
    /// Flattened shadow decals (the net's, `main`): drawn black, alpha = how much the shadow darkens.
    pub decals: HashSet<AssetId<GsMaterial>>,
    /// `_acc` and `_clo` models.
    pub acc: Vec<Entity>,
    pub clo: Vec<Entity>,
    pub last: Option<u8>,
    /// The weather `last` is fading towards and how far (0..1) the materials show it, as `Fade` had it.
    pub mix: Option<(u8, f32)>,
}

/// B360: the court's look fading towards the next game's weather over a change of ends (`play::weather`), so the
/// picture doesn't step when the new weather lands on the next point's set-up. Presentation only: the weather the
/// game plays (`Weather`, rain's agility, rain drops, clouds, background models) still changes in one tick there.
#[derive(Resource, Default, PartialEq)]
pub struct Fade(pub Option<(u8, f32)>);

pub fn plugin(app: &mut App) {
    app.init_resource::<Fade>().add_systems(Update, apply);
}

/// One weather's court look, as `apply` writes it: the fogs (main, bg, sky, colour), the light (colour, ambient,
/// second), the ground shadow's darkening and the clear colour.
#[derive(Clone, Copy)]
struct Lit {
    fog: Option<(Vec4, Vec4, Vec4, Vec4)>,
    light: Option<(Vec4, Vec4, Vec4)>,
    darken: f32,
    clear: Option<[f32; 3]>,
}

impl Lit {
    fn of(look: &CourtLook, w: u8, sun: Option<&crate::shadow::Sun>) -> Lit {
        let l = look.looks.get(w as usize).map_or(weather::Look::CLEAR, |r| weather::look(w, *r));
        let intensity = gs::court_intensity(&look.envir, &l).unwrap_or(0.0);
        Lit {
            fog: gs::court_fog(&look.envir, &l),
            light: gs::court_light(&look.envir, &l),
            darken: sun.map_or(0.0, |s| crate::shadow::darken_s(weather::shadow(crate::shadow::strength(s.strength, intensity), w))),
            clear: look.sky.and_then(|c| gs::court_clear(&look.envir, c, &l)).map(|c| c.map(f32::from)),
        }
    }

    fn mix(self, to: Lit, t: f32) -> Lit {
        let v = |a: Vec4, b: Vec4| a.lerp(b, t);
        Lit {
            fog: self.fog.zip(to.fog).map(|(a, b)| (v(a.0, b.0), v(a.1, b.1), v(a.2, b.2), v(a.3, b.3))).or(self.fog),
            light: self.light.zip(to.light).map(|(a, b)| (v(a.0, b.0), v(a.1, b.1), v(a.2, b.2))).or(self.light),
            darken: self.darken + (to.darken - self.darken) * t,
            clear: self.clear.zip(to.clear).map(|(a, b)| std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)).or(self.clear),
        }
    }
}

/// The fade's target and weight if it is towards another weather than `w`.
pub fn mix(fade: Option<&Fade>, w: u8) -> Option<(u8, f32)> {
    fade.and_then(|f| f.0).filter(|&(to, t)| to != w && t > 0.0)
}

pub fn apply(
    mut commands: Commands,
    weather: Option<Res<Weather>>,
    look: Option<ResMut<CourtLook>>,
    fade: Option<Res<Fade>>,
    sun: Option<Res<crate::shadow::Sun>>,
    mut materials: ResMut<Assets<GsMaterial>>,
    mut vis: Query<&mut Visibility>,
) {
    let (Some(weather), Some(mut look)) = (weather, look) else { return };
    let w = weather.today().weather;
    NOW.store(w, Ordering::Relaxed);
    let mix = mix(fade.as_deref(), w);
    if look.last == Some(w) && look.mix == mix {
        return;
    }
    let changed = look.last != Some(w);
    (look.last, look.mix) = (Some(w), mix);
    let mut lit = Lit::of(&look, w, sun.as_deref());
    if let Some((to, t)) = mix {
        lit = lit.mix(Lit::of(&look, to, sun.as_deref()), t);
    }
    let light = lit.light.zip(sun.as_deref());
    for (id, m) in materials.iter_mut() {
        if let Some((main, bg, sky, colour)) = lit.fog {
            m.uniform.fog = if look.skies.contains(&id) { sky } else if look.bg.contains(&id) { bg } else { main };
            m.uniform.fog_color = colour;
        }
        if let Some(((colour, ambient, second), sun)) = light {
            (m.uniform.light_dir, m.uniform.light_color, m.uniform.ambient) = (sun.light.extend(0.0), colour, ambient);
            (m.uniform.light2_dir, m.uniform.light2_color) = (-sun.light.extend(0.0), second);
            // the sky's light block has no glare; none in rain
            m.uniform.glare = (w < 2 && !look.skies.contains(&id)) as u8 as f32;
            // every bg-dir model (`_bg`, `_acc`, `_clo`, `_sky`) has VU1's unlit flag: the court load builds them
            // with object flags 2 (not in the file); checked exact on slot 4's GS dump (research/p17s6_unlit_gs.py)
            if look.skies.contains(&id) || look.bg.contains(&id) {
                m.uniform.unlit = 1.0;
            }
        }
        if look.holes.contains(&id) {
            m.uniform.shadow = lit.darken;
        }
        if look.decals.contains(&id) {
            m.uniform.color.w = lit.darken;
        }
    }
    if let Some([r, g, b]) = lit.clear {
        commands.insert_resource(ClearColor(Color::srgb_u8(r as u8, g as u8, b as u8)));
    }
    if !changed {
        return;
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

impl crate::Wire for Weather {}
