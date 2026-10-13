//! Gameplay simulation, ported for bit-faithful behaviour. Units: metres, 60 Hz frames, game space is Y-down.

pub mod ball;
pub mod params;
pub mod ps2;
pub mod rng;
pub mod shot;
pub mod contact;
pub mod quat;
pub mod score;
pub mod judge;
pub mod flow;
pub mod swing;
pub mod serve;
pub mod camera;
pub mod vu0;
pub mod libm;
/// Platform-independent `sinf`, `tanf`, `atanf`… for ports that don't use the game's own (`libm`): never std's.
pub use musl;
pub mod replay;
pub mod world;
pub mod mesh;
pub mod court;
pub mod player;
pub mod pose;
pub mod motion;
pub mod face;
pub mod effect;
pub mod court_anim;
pub mod cutaway;
pub mod sound;
pub mod umpire;
pub mod npc;
pub mod ai;
pub mod mt;
pub mod aim;
pub mod rally;
pub mod position;
pub mod clouds;
pub mod surprise;
pub mod bodyhit;
pub mod stats;
pub mod weather;
pub mod noise;
pub mod foot;
pub mod tornado;
pub mod shade;
pub mod instant;

/// serde has no arrays past 32 (`#[serde(with = "crate::big")]`): such an array as a sequence.
pub mod big {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

    pub fn serialize<S: Serializer, T: Serialize, const N: usize>(a: &[T; N], s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(a)
    }

    pub fn deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de>, const N: usize>(d: D) -> Result<[T; N], D::Error> {
        Vec::<T>::deserialize(d)?.try_into().map_err(|_| D::Error::custom(format!("not {N} entries")))
    }
}
