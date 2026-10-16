//! The match phase as the sim's mover and AI number it (`hst_sim::rally::phase`).

use super::Phase;
use hst_sim::rally::phase;

impl Phase {
    /// The phase's number (`player::Scene::phase`, `Body::toward`, `stick_dir`).
    pub(super) fn code(&self) -> u8 {
        match self {
            Phase::ChangeEnds(_) => phase::CHANGE_ENDS,
            Phase::Serve => phase::SERVE,
            Phase::Rally => phase::RALLY,
            Phase::Post => phase::POINT_OVER,
        }
    }
}
