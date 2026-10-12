//! Online match setup (M2e): the host picks the match seed, and every peer's handshake must agree with the host's on
//! everything that changes the simulation before the match starts, else it is refused with a message saying what
//! differs. The transport (M2f) sends `Handshake`s; this is what they hold and how they're compared.

use crate::mods::{MatchMod, Mod};
use bevy::prelude::*;
use hst_sim::rng::Seed;

/// The match seed to play with, when it isn't this app's to pick (a peer takes the host's): `load` uses it instead
/// of picking one.
#[derive(Resource, Clone, Copy)]
pub struct MatchSeed(pub Seed);

/// What a peer brings to a match: its build, its disc, the match as it understands it and its mods' gameplay data.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct Handshake {
    pub build: &'static str,
    /// The disc pack's hash (`Iso::pack_hash`); `None` running from an ISO (its disc ID is checked at start).
    pub pack: Option<u64>,
    pub court: usize,
    pub singles: bool,
    pub sets: i32,
    pub games: i32,
    pub umpire: u8,
    /// Which peer plays each seat (`None` the computer). Offline every human seat is peer 0's; the lobby fills these.
    pub seats: [Option<u8>; 4],
    pub chars: Vec<usize>,
    pub outfits: Vec<usize>,
    /// Each match slot's mod gameplay hash (`gameplay_hash`), `None` for a disc character.
    pub mods: Vec<Option<u64>>,
    /// The host's pick; a peer's own is ignored.
    pub seed: Seed,
}

/// The build: two builds can't be trusted to simulate alike. `HST_BUILD` (e.g. a commit id) set at compile time
/// tells apart builds of one version.
// ponytail: the crate version alone without HST_BUILD; a build.rs with the git commit if dev builds play online
pub const BUILD: &str = match option_env!("HST_BUILD") {
    Some(b) => b,
    None => env!("CARGO_PKG_VERSION"),
};

impl Handshake {
    pub fn of(args: &crate::Args, pack: Option<u64>, mods: Option<&MatchMod>, seed: Seed) -> Handshake {
        let seats = args.pads.map_or([None; 4], |p| p.map(|s| s.map(|_| 0)));
        let mods = (0..args.chars.len()).map(|n| mods.and_then(|m| m.get(n)).map(gameplay_hash)).collect();
        Handshake {
            build: BUILD,
            pack,
            court: args.court,
            singles: args.singles,
            sets: args.sets,
            games: args.games,
            umpire: args.umpire,
            seats,
            chars: args.chars.clone(),
            outfits: args.outfits.clone(),
            mods,
            seed,
        }
    }

    /// A joining peer's handshake (`self`) against the host's: `Ok` to play, else why not, in words for the player.
    #[allow(dead_code)] // the transport (M2f) calls it
    pub fn check(&self, host: &Handshake) -> Result<(), String> {
        let differ = |what: &str, mine: String, theirs: String| Err(format!("Can't join: {what} differs (host: {theirs}, you: {mine})."));
        if self.build != host.build {
            return differ("the game version", self.build.into(), host.build.into());
        }
        if let (Some(a), Some(b)) = (self.pack, host.pack)
            && a != b
        {
            return Err(format!(
                "Can't join: your imported disc ({a:016x}) isn't the same as the host's ({b:016x}). Re-import an unmodified Hot Shots Tennis (USA) disc."
            ));
        }
        let rules = |h: &Handshake| {
            let sets = if h.sets == 1 { "1 set".to_string() } else { format!("{} sets", h.sets) };
            format!("court {}, {}, {sets} of {} games, umpire {}", h.court, if h.singles { "singles" } else { "doubles" }, h.games, h.umpire)
        };
        if rules(self) != rules(host) {
            return differ("the match rules", rules(self), rules(host));
        }
        if self.seats != host.seats {
            return differ("who plays which seat", format!("{:?}", self.seats), format!("{:?}", host.seats));
        }
        if self.chars != host.chars || self.outfits != host.outfits {
            let picks = |h: &Handshake| format!("characters {:?}, costumes {:?}", h.chars, h.outfits);
            return differ("the characters", picks(self), picks(host));
        }
        if let Some(n) = (0..self.mods.len().max(host.mods.len())).find(|&n| self.mods.get(n).copied().flatten() != host.mods.get(n).copied().flatten()) {
            return Err(format!(
                "Can't join: player {}'s mod plays differently for you and the host (its stats, hand or AI aren't the same). Use the same version of the mod.",
                n + 1
            ));
        }
        Ok(())
    }
}

/// A mod's gameplay data (what feeds the simulation: donor, hand, TParam row and overrides, AI row) as a 64-bit
/// FNV-1a hash, the same on every platform. Looks (models, textures, faces, voice, motions drawn) aren't in it.
pub fn gameplay_hash(m: &Mod) -> u64 {
    let s = format!("{} {} {} {:?} {}", m.donor, m.hand.to_bits(), m.base, m.overrides, m.ai_row);
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> Handshake {
        Handshake {
            build: BUILD,
            pack: Some(7),
            court: 3,
            singles: true,
            sets: 1,
            games: 4,
            umpire: 4,
            seats: [Some(0), Some(1), None, None],
            chars: vec![0, 2],
            outfits: vec![9, 9],
            mods: vec![None, Some(5)],
            seed: Seed::after(10),
        }
    }

    #[test]
    fn same_match_joins_and_differences_are_refused() {
        let h = host();
        // the joining peer's own seed doesn't matter, and an ISO (no pack hash) is taken on its disc ID
        assert_eq!(Handshake { seed: Seed::after(0), pack: None, ..h.clone() }.check(&h), Ok(()));
        let refused = |p: Handshake, says: &str| {
            let e = p.check(&h).unwrap_err();
            assert!(e.contains(says), "{e}");
        };
        refused(Handshake { build: "0.0.0-other", ..h.clone() }, "game version");
        refused(Handshake { pack: Some(8), ..h.clone() }, "imported disc");
        refused(Handshake { games: 6, ..h.clone() }, "6 games");
        refused(Handshake { court: 1, ..h.clone() }, "match rules");
        refused(Handshake { seats: [Some(1), Some(0), None, None], ..h.clone() }, "seat");
        refused(Handshake { outfits: vec![9, 4], ..h.clone() }, "costumes");
        refused(Handshake { mods: vec![None, Some(6)], ..h.clone() }, "player 2's mod");
        refused(Handshake { mods: vec![None, None], ..h.clone() }, "player 2's mod");
    }

    #[test]
    fn mod_hash_follows_gameplay_data_only() {
        let m = Mod {
            dir: "x".into(),
            id: "x".into(),
            name: "X".into(),
            costumes: vec!["model/c00.glb".into()],
            donor: 3,
            hand: 1.0,
            base: 3,
            overrides: vec![("Strk POW".into(), "5".into())],
            ai_row: 3,
            texture_face: false,
            no_face: false,
            voice: "voice/".into(),
            motions: None,
            skeleton: None,
        };
        let h = gameplay_hash(&m);
        assert_eq!(h, gameplay_hash(&Mod { name: "Y".into(), costumes: vec![], voice: "v/".into(), ..m.clone() }));
        assert_ne!(h, gameplay_hash(&Mod { hand: -1.0, ..m.clone() }));
        assert_ne!(h, gameplay_hash(&Mod { ai_row: 4, ..m.clone() }));
        assert_ne!(h, gameplay_hash(&Mod { overrides: vec![("Strk POW".into(), "6".into())], ..m.clone() }));
    }
}
