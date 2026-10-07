//! Background figures: which of a court's creature records (layout category 23) come to life at court load, as
//! what, and where they stand. A creature linked to an anchor record (category 14) stands at the anchor's position
//! minus its own (FPU subtractions); every figure's matrix is a turn by its yaw at that position.

use crate::ps2;
use crate::world::{self, M4};
use hst_data::exe::NpcEntry;
use hst_data::layout::{self, Entry, Placement};

/// What a creature record becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A trigger creature of this type (0..53); only made with an anchor.
    Trigger(u8),
    /// Walking spectator 0..5 (npc00..npc05).
    Walker(u8),
    /// The umpire's chair (made when more than one player).
    Umpire,
    /// One of court 5's own creatures (0..5).
    Court5(u8),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Npc {
    /// Index among the court's category-23 records, in file order.
    pub record: usize,
    pub kind: Kind,
    pub world: M4,
    pub scale: f32,
}

/// Where each category-23 record stands, in file order (links resolved against the anchors' positions).
pub fn positions(plants: &[Placement]) -> Vec<[f32; 3]> {
    let of = |cat: u8, i: u16| plants.iter().filter(|p| p.category == cat).nth(i as usize).map(|p| p.pos);
    plants
        .iter()
        .filter(|p| p.category == 23)
        .map(|p| match p.link.and_then(|(c, i)| of(c, i)) {
            Some(a) => std::array::from_fn(|k| ps2::sub(a[k], p.pos[k])),
            None => p.pos,
        })
        .collect()
}

/// The figures made at court load. `entries`/`plants` are the court's layout, `roster`/`walkers` its rows of
/// the game's tables (`exe::Game::npc_roster` / `walkers`), `players` the number of players on court.
pub fn spawn(entries: &[Entry], plants: &[Placement], roster: &[NpcEntry], walkers: &[[u8; 3]; 6], players: u32) -> Vec<Npc> {
    let pos = positions(plants);
    plants
        .iter()
        .filter(|p| p.category == 23)
        .enumerate()
        .filter_map(|(record, p)| {
            let stem = &layout::resolve(entries, p, 0)?.stem;
            let kind = match roster.iter().find(|e| e.name.eq_ignore_ascii_case(stem))?.kind {
                // ponytail: the anchor object exists for every category-14 link in the courts checked
                t @ 0..=53 => (p.link?.0 == 14).then_some(Kind::Trigger(t))?,
                t @ 54..=59 => (players < 3 || walkers[t as usize - 54][2] != 0).then_some(Kind::Walker(t - 54))?,
                60 => (players >= 2).then_some(Kind::Umpire)?,
                t => Kind::Court5(t - 61),
            };
            let mut world = world::mat_mul(&world::IDENTITY, &world::rot_y(p.yaw));
            world[3] = [pos[record][0], pos[record][1], pos[record][2], 1.0];
            Some(Npc { record, kind, world, scale: p.scale })
        })
        .collect()
}
