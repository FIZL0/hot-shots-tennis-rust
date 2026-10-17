//! B523: the host's say over the lobby's players, once the host has picked: any other person's card (a guest's, the
//! host's own 2P..4P) can be picked up (✕), carried to another card (the arrows, or a tap) and put down on it (✕) to
//! swap those two seats, rearranging the teams (an open seat or a computer's: the person moves there); ○ puts it
//! back. A computer's card isn't picked up: its ✕ picks its character, as on the roster (B216). △ on a guest's card
//! asks to kick them (a computer's goes off at once, B216). Guests can do neither: the seats are the host's
//! (`Cmd::Swap` is only heard there).

use super::super::{Effect, Menu, Screen};
use super::{Fx, Item, Press, name_of, pill_step};
use crate::net::Cmd;

/// The kick prompt's rows; Kick is the first.
pub(super) const KICK_ROWS: [&str; 2] = ["Kick", "Cancel"];

impl Menu {
    /// The host's lobby with the host's own player picked and no countdown running.
    fn host_controls(&self) -> bool {
        let Some(r) = self.net.room.as_ref().filter(|_| self.net.hosting && !self.net.started && self.screen == Screen::Lobby) else { return false };
        let own = r.seats.iter().position(|s| s.as_ref().is_some_and(|s| s.peer == "host"));
        self.counting() == 0 && own.is_some_and(|i| super::lobby::state(r, i) != super::lobby::State::Picking)
    }

    /// Seat `i`'s card is one the host may pick up: a person's other than the host's own (its first seat); not a
    /// computer's (its ✕ picks its character).
    pub(in super::super) fn movable(&self, i: usize) -> bool {
        let Some(r) = self.net.room.as_ref().filter(|_| self.host_controls()) else { return false };
        let own = r.seats.iter().position(|s| s.as_ref().is_some_and(|s| s.peer == "host"));
        r.seats.get(i).is_some_and(|s| s.as_ref().is_some_and(crate::net::Seat::human)) && own != Some(i)
    }

    /// The chosen lobby item is a guest's card the host may kick (△).
    pub(in super::super) fn kickable(&self) -> Option<usize> {
        let i = match self.items().get(self.sel) {
            Some(&Item::Seat(i)) => i,
            _ => return None,
        };
        let r = self.net.room.as_ref().filter(|_| self.host_controls())?;
        r.seats.get(i)?.as_ref().filter(|s| s.human() && s.peer != "host").map(|_| i)
    }

    /// The prompt to kick seat `i`'s player (the lobby's cursor kept for after).
    pub(super) fn ask_kick(&mut self, i: usize, out: &mut Vec<Effect>) {
        self.net.kick = Some((i, self.sel));
        self.net.grab = None;
        self.sel = 1;
        out.push(Effect::Sound(2));
    }

    /// The kick prompt's title and help: who, and how many of theirs go.
    pub(super) fn kick_words(&self) -> Option<(String, String)> {
        let (i, _) = self.net.kick?;
        let r = self.net.room.as_ref()?;
        let peer = &r.seats.get(i)?.as_ref()?.peer;
        let name = name_of(r, peer);
        let n = r.seats.iter().flatten().filter(|s| s.peer == *peer).count();
        let help = match n {
            1 => format!("{name} leaves the lobby; their seat opens."),
            n => format!("{name} and their {n} players leave the lobby; their seats open."),
        };
        Some((format!("Kick {name}?"), help))
    }

    /// A press on the kick prompt: ↑↓ a row, ✕ Kick or Cancel, ○ Cancel.
    pub(super) fn kick_step(&mut self, k: Press, out: &mut Vec<Effect>) {
        let Some((i, back)) = self.net.kick else { return };
        if k.up || k.down {
            self.sel = 1 - self.sel.min(1);
            return out.push(Effect::Sound(0));
        }
        if !k.ok && !k.back {
            return;
        }
        let peer = self.net.room.as_ref().and_then(|r| r.seats.get(i)?.as_ref()).filter(|s| s.human() && s.peer != "host").map(|s| s.peer.clone());
        let kick = k.ok && self.sel == 0;
        (self.net.kick, self.sel) = (None, back);
        match peer.filter(|_| kick) {
            Some(peer) => {
                let name = self.net.room.as_ref().map(|r| name_of(r, &peer)).unwrap_or_default();
                self.net.status = format!("Taking {name} out...");
                out.extend([Effect::Sound(1), Effect::Online(Fx::Net(Cmd::Kick(peer)))]);
            }
            None => out.push(Effect::Sound(1)),
        }
    }

    /// The card being carried, if it still is one the host may carry.
    pub(in super::super) fn carrying(&self) -> Option<usize> {
        self.net.grab.filter(|&i| self.movable(i))
    }

    /// Where the carried card would go: the chosen card's seat.
    pub(in super::super) fn target_seat(&self) -> Option<usize> {
        match self.items().get(self.sel) {
            Some(&Item::Seat(t)) => Some(t),
            _ => None,
        }
    }

    /// ✕ on movable card `i`: picked up.
    pub(super) fn pick_up(&mut self, i: usize, out: &mut Vec<Effect>) {
        self.net.grab = Some(i);
        self.net.status.clear();
        out.push(Effect::Sound(2));
    }

    /// A press while a card is carried: the arrows go between the cards, ✕ puts it down there (on itself: back where
    /// it was), ○ puts it back.
    pub(super) fn carry_step(&mut self, k: Press, out: &mut Vec<Effect>) {
        let Some(from) = self.carrying() else {
            self.net.grab = None;
            return;
        };
        let items = self.items();
        let cards: Vec<usize> = (0..items.len()).filter(|&j| matches!(items[j], Item::Seat(_))).collect();
        let here = self.target_seat().and_then(|t| cards.iter().position(|&j| items[j] == Item::Seat(t))).unwrap_or(0);
        if (k.left || k.right || k.up || k.down) && !k.ok {
            let rects = self.lobby_rects();
            let sub: Vec<[f32; 4]> = cards.iter().map(|&j| rects[j]).collect();
            let to = cards[pill_step(&sub, here, k)];
            if to != self.sel {
                self.sel = to;
                out.push(Effect::Sound(0));
            }
            return;
        }
        let back_on = |m: &mut Menu, i: usize| m.sel = m.items().iter().position(|x| *x == Item::Seat(i)).unwrap_or(m.sel);
        if k.back {
            self.net.grab = None;
            back_on(self, from);
            return out.push(Effect::Sound(1));
        }
        if !k.ok {
            return;
        }
        let Some(to) = self.target_seat() else { return out.push(Effect::Sound(1)) };
        if to == from {
            self.net.grab = None;
            return out.push(Effect::Sound(1));
        }
        let Some(r) = self.net.room.as_mut() else { return };
        if !r.can_swap(from, to) {
            let name = r.seats[from].as_ref().map(|s| name_of(r, &s.peer)).unwrap_or_default();
            self.net.status = format!("{name}'s players keep their order: move the other one.");
            return out.push(Effect::Sound(1));
        }
        // the host's room has it at once (its own loop does the same and sends it round): the card is where it went
        r.seats.swap(from, to);
        self.net.grab = None;
        self.net.status.clear();
        self.align();
        back_on(self, to);
        out.extend([Effect::Sound(2), Effect::Online(Fx::Net(Cmd::Swap(from, to)))]);
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{Dev, Screen, legend};
    use super::super::{Fx, Item, Press, sample};
    use super::*;
    use crate::net::{CPU, Pick, Seat};
    use bevy::prelude::default;

    fn net(fx: &[Effect]) -> Option<Cmd> {
        fx.iter().find_map(|e| if let Effect::Online(Fx::Net(c)) = e { Some(c.clone()) } else { None })
    }

    fn to(m: &mut Menu, it: Item) {
        m.sel = m.items().iter().position(|x| *x == it).unwrap();
    }

    fn says(m: &Menu, words: &str) -> bool {
        legend::acts(m, Dev::Keys).iter().any(|(_, w)| w == words)
    }

    /// The host carries Sunny Day (2P) onto the open 4P, then onto ACE-42 (3P), and kicks ACE-42 once confirmed; a
    /// guest can do neither.
    #[test]
    fn host_moves_and_kicks_cards() {
        let (dev, ok, back) = (Dev::Keys, Press { ok: true, ..default() }, Press { back: true, ..default() });
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        (m.net.me, m.seats[0]) = ("host".into(), Some(dev));
        // the host's own card isn't carried: ✕ picks again
        to(&mut m, Item::Seat(0));
        assert!(!m.movable(0) && m.kickable().is_none() && m.lobby_ok() == Some("Change Character"));
        to(&mut m, Item::Seat(1));
        assert!(says(&m, "Move") && says(&m, "Kick"), "{:?}", legend::acts(&m, Dev::Keys));
        assert_eq!(net(&m.step(dev, ok)), None);
        assert_eq!(m.carrying(), Some(1));
        assert!(says(&m, "Put Back") && says(&m, "Cancel"));
        // the arrows go between the cards only
        for _ in 0..8 {
            m.step(dev, Press { right: true, ..default() });
            assert!(matches!(m.items()[m.sel], Item::Seat(_)), "{:?}", m.items()[m.sel]);
        }
        to(&mut m, Item::Seat(3));
        assert!(says(&m, "Drop"));
        let fx = m.step(dev, ok);
        assert_eq!(net(&fx), Some(Cmd::Swap(1, 3)));
        let r = m.net.room.as_ref().unwrap();
        assert!(r.seats[1].is_none() && r.seats[3].as_ref().is_some_and(|s| s.peer == "k51qzi5uqu5d"));
        assert_eq!((m.items()[m.sel], m.carrying()), (Item::Seat(3), None), "on the card where it went");
        // onto another player: swapped; onto a peer's other seat: refused
        m.step(dev, ok);
        to(&mut m, Item::Seat(2));
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Swap(3, 2)));
        let r = m.net.room.as_mut().unwrap();
        assert_eq!(r.seats.iter().map(|s| s.as_ref().map(|s| s.peer.clone())).collect::<Vec<_>>(), [Some("host".into()), None, Some("k51qzi5uqu5d".into()), Some("r8mxw2pa0c1e".into())]);
        r.seats[1] = Some(Seat { peer: "r8mxw2pa0c1e".into(), pick: Pick::disc(3, 0) });
        to(&mut m, Item::Seat(1));
        m.step(dev, ok);
        to(&mut m, Item::Seat(3));
        assert_eq!((net(&m.step(dev, ok)), m.carrying()), (None, Some(1)), "ACE-42's two");
        assert!(m.net.status.contains("order"), "{}", m.net.status);
        m.step(dev, back);
        m.net.room.as_mut().unwrap().seats[1] = None;
        // △ on a guest: asked first (on Cancel); ○ or Cancel keeps them, Kick takes them out
        to(&mut m, Item::Seat(3));
        assert_eq!((net(&m.step(dev, Press { card: true, ..default() })), m.lobby_asking()), (None, true));
        assert_eq!(m.kick_words().unwrap().0, "Kick ACE-42?");
        assert_eq!((net(&m.step(dev, back)), m.net.kick, m.items()[m.sel]), (None, None, Item::Seat(3)));
        m.step(dev, Press { card: true, ..default() });
        assert_eq!(net(&m.step(dev, ok)), None, "Cancel");
        m.step(dev, Press { card: true, ..default() });
        m.step(dev, Press { down: true, ..default() });
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Kick("r8mxw2pa0c1e".into())));
        // a computer's card isn't picked up: ✕ picks its character (as on the roster), △ takes it off at once (B216)
        m.net.room.as_mut().unwrap().seats[3] = Some(Seat { peer: CPU.into(), pick: Pick::disc(9, 4) });
        to(&mut m, Item::Seat(3));
        assert!(!m.movable(3) && m.kickable().is_none() && m.lobby_ok() == Some("Pick") && says(&m, "Remove"));
        assert_eq!(net(&m.step(dev, Press { card: true, ..default() })), Some(Cmd::Bot(3, None)));
        m.net.room.as_mut().unwrap().seats[3] = Some(Seat { peer: CPU.into(), pick: Pick::disc(9, 4) });
        m.step(dev, ok);
        assert_eq!((m.screen, m.net.edit, m.net.grab), (Screen::Chars, Some(3), None));
        (m.screen, m.net.edit) = (Screen::Lobby, None);
        // a person dropped on it: they trade seats
        m.net.room.as_mut().unwrap().seats[1] = Some(Seat { peer: "k51qzi5uqu5d".into(), pick: Pick::disc(1, 0) });
        to(&mut m, Item::Seat(1));
        m.step(dev, ok);
        to(&mut m, Item::Seat(3));
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Swap(1, 3)), "onto the computer's seat");
        // not before the host has picked, nor during the countdown
        m.net.room.as_mut().unwrap().picking.insert("host".into());
        assert!((0..4).all(|i| !m.movable(i)));
        m.net.room.as_mut().unwrap().picking.clear();
        m.net.room.as_mut().unwrap().count = 3;
        assert!((0..4).all(|i| !m.movable(i)));
        // a guest: no carrying, no kicking
        let mut g = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut g);
        (g.net.hosting, g.net.me) = (false, "k51qzi5uqu5d".into());
        g.align();
        to(&mut g, Item::Seat(1));
        assert!((0..4).all(|i| !g.movable(i)) && g.kickable().is_none() && !says(&g, "Kick"));
        let fx = g.step(dev, Press { card: true, ..default() });
        assert!(net(&fx).is_none() && g.net.kick.is_none() && g.net.grab.is_none());
    }
}
