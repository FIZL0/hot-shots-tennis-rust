//! B196: a peer that goes silent mid-match (crash, lost network, closed tab, frozen machine). Every machine sends a
//! keep-alive ([`ALIVE`]); the host, hearing nothing from a guest for [`SILENT`] seconds, says it's away
//! (`Msg::Away`) and everyone shows "Waiting for …" with a countdown, the match held where it is (rollback stalls
//! at its window without the peer's pads, lockstep at once). The same peer heard again: the match just goes on. A
//! new process joining with the lobby's code takes the away player's seats (`net.rs` relay), and the host restarts
//! the match with it in. After [`WAIT`] the players are asked: a bot takes the seats, or everyone still in goes
//! back to the lobby together (`Msg::Lobby`; no answer in [`CHOOSE`]: the lobby too), or this one leaves alone.
//!
//! A restart (`Msg::Restart`) is the host's state before a tick every machine has every seat's pads for (rollback:
//! ggrs' confirmed tick; lockstep: its next tick) and the new line-up; every machine, the host too, takes the state
//! on, sets which seats are people (`Pads::seats`: a bot's is the computer's from that tick, as a seat started empty
//! is) and starts a new rollback session (or lockstep) from that tick. The host silent: the guests wait
//! [`HOST_WAIT`] and end the match (the star goes through it; there's no new host).
//!
//! B162: a player who switched away (a hidden tab, a minimised window: its frames stop) isn't silent: its machine's
//! keeper (`net::keeper`) says so (`Msg::Hidden`) about once a second. Everyone holds the match for it with "… switched
//! away" and a countdown of [`LONG`]; it's away (the wait and question above) only after that, or if even the keeper
//! goes quiet for [`HIDDEN_QUIET`] (a browser throttles a hidden tab's timers). Back, it plays on from where it was.
//!
//! M126: a guest's page refreshed or game restarted comes back by itself: its lobby code and seat are kept
//! (`net::rejoin`, the menu's `launch`) and a fresh menu joins that code again, asking for its old seat. [`WAIT`] covers
//! a phone reloading the page and the disc pack. The record goes once the match is over here. A refreshed host still
//! ends the match: the guests' connections were to it, and there's no new host.

use super::netplay::{Lockstep, Netplay, unwire};
use super::tick_input::IDLE;
use super::{Game, Pads, snapshot};
use crate::net::{CPU, Event, Msg, Room, Seat};
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// Keep-alive every this many seconds.
const ALIVE: f32 = 0.5;
/// A peer not heard for this long is away; before it has loaded the match (`Msg::Loaded`: its first word may come
/// before its loading screen's slow frames, a browser compiling pipelines), [`LOADING`] (B200).
pub const SILENT: f32 = 3.0;
const LOADING: f32 = 60.0;
/// How long the others wait for an away player to come back (M126: long enough for a refreshed page or a restarted
/// game to load and rejoin), then for an answer. The match tests (`netplay::tests::matches`) wait 20 s, as before M126,
/// so they don't play 100 s more of frames.
#[cfg(not(test))]
pub const WAIT: f32 = 120.0;
#[cfg(test)]
pub const WAIT: f32 = 20.0;
pub const CHOOSE: f32 = 15.0;
/// How long the guests wait for a silent host.
const HOST_WAIT: f32 = 10.0;
/// B162: how long everyone waits for a player who switched away, and how long its keeper may go unheard meanwhile.
pub const LONG: f32 = 300.0;
const HIDDEN_QUIET: f32 = 75.0;

pub struct Recover {
    /// Seconds since each peer was last heard, and whether it has been at all.
    quiet: BTreeMap<String, (f32, bool)>,
    alive: f32,
    /// The host: the peers away, seconds waited, the answer's countdown once it asks, an answer, the countdown sent.
    pub(super) away: Vec<String>,
    waited: f32,
    asking: Option<f32>,
    chosen: Option<bool>,
    sent: Option<(u32, bool)>,
    /// A guest: the host's last word (who, seconds left, asking) and whether this machine answered it.
    told: Option<(Vec<String>, u32, bool)>,
    answered: bool,
    /// A restart's pieces so far, and a whole one to take on.
    pieces: Vec<u8>,
    ready: Option<(u8, u32, Room, [[u8; 6]; 4], Vec<u8>)>,
    /// The restarts so far.
    round: u8,
    /// Hold the match until a restart (a rejoiner).
    pub hold: bool,
    /// Back to this lobby once the end notice is down (`netplay::LOBBY`).
    pub lobby: Option<Room>,
    /// This frame's notice asks this machine bot-or-lobby (B196a: its pills, `main_menu/pause.rs`).
    pub ask_me: bool,
    /// B200: the peers (this machine too) past their loading screen; the match's ticks wait for all of them.
    pub loaded: BTreeSet<String>,
    /// B162: the peers whose players switched away, seconds since; this machine's keeper (`keep`).
    pub hidden: BTreeMap<String, f32>,
    keeper: Option<crate::net::keeper::Keeper>,
    /// M126: this machine's rejoin record is gone (the match is over).
    forgot: bool,
}

impl Recover {
    /// `others`: the other peers, already playing if this machine rejoins.
    pub fn new(rejoin: bool, others: impl Iterator<Item = String>) -> Self {
        let loaded = if rejoin { others.collect() } else { BTreeSet::new() };
        Recover { quiet: BTreeMap::new(), alive: 0.0, away: Vec::new(), waited: 0.0, asking: None, chosen: None, sent: None, told: None, answered: false, pieces: Vec::new(), ready: None, round: 0, hold: rejoin, lobby: None, ask_me: false, loaded, hidden: BTreeMap::new(), keeper: None, forgot: false }
    }

    /// Any word from a peer.
    pub fn heard_from(&mut self, e: &Event) {
        if let Event::Datagram(p, _) | Event::Match(Msg::Alive(p) | Msg::Loaded(p) | Msg::Hidden(p)) = e {
            self.quiet.insert(p.clone(), (0.0, true));
        }
        // B162: the keeper while it's away, the frame loop's keep-alive once it's back
        match e {
            Event::Match(Msg::Hidden(p)) if !self.hidden.contains_key(p) => {
                info!("online: {p} switched away");
                self.hidden.insert(p.clone(), 0.0);
            }
            Event::Match(Msg::Alive(p)) if self.hidden.remove(p).is_some() => info!("online: {p} is back"),
            _ => {}
        }
        if let Event::Match(Msg::Loaded(p)) = e {
            self.loaded.insert(p.clone());
        }
    }

    /// B200: the people in `room` who haven't loaded the match yet (empty: it runs).
    pub fn loading(&self, room: &Room) -> Vec<String> {
        let mut v: Vec<String> = room.seats.iter().flatten().filter(|s| s.human() && !self.loaded.contains(&s.peer)).map(|s| s.peer.clone()).collect();
        v.dedup();
        v
    }

    /// P52: `loading` is empty, without listing who (asked every tick).
    pub fn all_loaded(&self, room: &Room) -> bool {
        !room.seats.iter().flatten().any(|s| s.human() && !self.loaded.contains(&s.peer))
    }
}

/// A recovery message in.
pub(super) fn heard(np: &mut Netplay, m: Msg) {
    let host = np.me == "host";
    let mut room = np.room.clone();
    let r = &mut np.recover;
    match m {
        Msg::Away { who, left, ask } if !host => {
            r.answered &= ask;
            r.told = (!who.is_empty()).then_some((who, left, ask));
        }
        Msg::Choose(bot) if host && r.asking.is_some() => {
            r.chosen.get_or_insert(bot);
        }
        Msg::Restart { round, tick, room, last, data, end } if !host => {
            r.pieces.extend(data);
            if end {
                r.ready = Some((round, tick, room, last, std::mem::take(&mut r.pieces)));
            }
        }
        Msg::Lobby(room) if !host => {
            let who = r.told.as_ref().map_or(String::new(), |t| t.0.iter().map(|p| name(&np.room, p)).collect::<Vec<_>>().join(", "));
            r.lobby = Some(room);
            np.finish(format!("{who} didn't come back. Back to the lobby..."), false);
        }
        Msg::Rejoined { old, new, name } if host => {
            info!("online: {name} is back (was {old}, now {new})");
            r.away.retain(|p| *p != old);
            r.quiet.insert(new.clone(), (0.0, true));
            room.rejoined(&old, &new, &name);
            r.chosen = None;
            // the restart goes out at the next gate or frame (it needs the world)
            r.ready = Some((0, u32::MAX, room, [[0; 6]; 4], Vec::new()));
        }
        _ => {}
    }
}

fn name(room: &Room, p: &str) -> String {
    room.names.get(p).cloned().unwrap_or_else(|| p.chars().take(8).collect())
}

/// The score so far, for the end of a match that couldn't go on.
fn score(w: &World) -> String {
    let Some(g) = w.get_resource::<Game>() else { return String::new() };
    let s = &g.score;
    format!("Score (1P's side first): sets {}-{}, games {}-{}.", s.sets[0], s.sets[1], s.games[0], s.games[1])
}

/// The answer chosen this frame on this machine, a `CHOICES` index: ✕ (or Enter, or a tap) on the chosen pill (up/down
/// move between them; a bot is pill 0, without the pills too), ○ (or Backspace) the lobby.
fn pressed(w: &mut World) -> Option<usize> {
    let keys = w.get_resource::<ButtonInput<KeyCode>>()?;
    let mut yes = keys.any_just_pressed([KeyCode::Enter, KeyCode::Space, KeyCode::KeyJ]);
    let mut no = keys.any_just_pressed([KeyCode::Backspace, KeyCode::KeyK]);
    let mut up = keys.any_just_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let mut down = keys.any_just_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let mut q = w.query::<&Gamepad>();
    for g in q.iter(w).filter(|g| super::controls::readable(g)) {
        yes |= g.just_pressed(GamepadButton::South);
        no |= g.just_pressed(GamepadButton::East);
        up |= g.just_pressed(GamepadButton::DPadUp);
        down |= g.just_pressed(GamepadButton::DPadDown);
    }
    let mut sel = 0;
    if let Some(mut n) = w.get_resource_mut::<super::netplay::OnlineNotice>().filter(|n| n.ask) {
        let k = super::netplay::CHOICES.len();
        n.sel = (n.sel + usize::from(down) + (k - 1) * usize::from(up)) % k;
        yes |= std::mem::take(&mut n.tapped);
        sel = n.sel;
    }
    if yes { Some(sel) } else if no { Some(1) } else { None }
}

/// P52: seconds since `p` was last heard; a peer not seen yet starts counting (its name copied only then).
fn quiet(quiet: &mut BTreeMap<String, (f32, bool)>, p: &str) -> f32 {
    match quiet.get(p) {
        Some(q) => q.0,
        None => {
            quiet.insert(p.to_string(), Default::default());
            0.0
        }
    }
}

/// Each frame (`dt` real seconds): the keep-alive out, who's silent, the countdowns and answers; the notice to show
/// (empty: none).
pub(super) fn tick(w: &mut World, dt: f32) -> String {
    restarted(w);
    let press = pressed(w);
    let score = score(w);
    // B279: on the result screen nobody waits for anybody: each leaves it for the lobby in their own time
    let over = w.get_resource::<Game>().is_some_and(|g| g.match_stats.result.is_some());
    // M126: over, nothing to rejoin after a refresh
    if (over || w.resource::<Netplay>().end.is_some()) && !w.resource::<Netplay>().recover.forgot {
        w.resource_mut::<Netplay>().recover.forgot = true;
        if let Some(a) = w.get_resource::<crate::Args>() {
            crate::net::rejoin::forget(&a.iso);
        }
    }
    let mut np = w.resource_mut::<Netplay>();
    let np = &mut *np;
    let r = &mut np.recover;
    r.ask_me = false;
    r.alive += dt;
    if r.alive >= ALIVE {
        r.alive = 0.0;
        np.out.push(Msg::Alive(np.me.clone()));
    }
    if np.end.is_some() || over {
        return String::new();
    }
    // Leave: this machine alone goes to the main menu; the host's leaving ends the match for everyone (no host)
    if press == Some(2) {
        if np.me == "host" {
            np.out.push(Msg::Bye(format!("{} left.", name(&np.room, "host"))));
        }
        np.finish("You left the match.".into(), false);
        return String::new();
    }
    let press = press.map(|c| c == 0);
    for q in r.quiet.values_mut() {
        q.0 += dt;
    }
    for h in r.hidden.values_mut() {
        *h += dt;
    }
    let names = |room: &Room, who: &[String]| who.iter().map(|p| name(room, p)).collect::<Vec<_>>().join(", ");
    if np.me != "host" {
        let q = quiet(&mut r.quiet, "host");
        let hid = r.hidden.get("host").copied();
        let limit = if hid.is_some() { HIDDEN_QUIET } else if r.loaded.contains("host") { SILENT } else { LOADING };
        if hid.is_some_and(|h| h > LONG) {
            np.finish(format!("{} switched away for {} minutes. The match is over.\n{score}", name(&np.room, "host"), LONG as u32 / 60), false);
            return String::new();
        }
        if q > limit + HOST_WAIT {
            np.finish(format!("The host stopped answering. The match is over.\n{score}"), false);
            return String::new();
        }
        if q > limit {
            return format!("Waiting for the host... {} s", (limit + HOST_WAIT - q).ceil());
        }
        let Some((who, left, ask)) = &r.told else { return switched(&np.room, &r.hidden) };
        let (left, ask) = (*left, *ask);
        if ask && !r.answered && let Some(bot) = press {
            r.answered = true;
            np.out.push(Msg::Choose(bot));
        }
        r.ask_me = ask && !r.answered;
        return notice(&names(&np.room, who), left, ask, r.answered);
    }
    // the host: who's away and who's back
    let mut last = None;
    for p in np.room.seats.iter().flatten().filter(|s| s.human() && s.peer != np.me).map(|s| s.peer.as_str()) {
        // consecutive seats of one peer count once
        if last.replace(p) == Some(p) {
            continue;
        }
        let q = quiet(&mut r.quiet, p);
        let away = r.away.iter().any(|x| x == p);
        let hid = r.hidden.get(p).copied();
        let limit = if hid.is_some() { HIDDEN_QUIET } else if r.loaded.contains(p) { SILENT } else { LOADING };
        let gone = q > limit || hid.is_some_and(|h| h > LONG);
        if !away && gone {
            info!("online: {} went silent", name(&np.room, p));
            r.away.push(p.to_string());
        } else if away && !gone && q < SILENT {
            info!("online: {} is answering again", name(&np.room, p));
            r.away.retain(|x| x != p);
        }
    }
    if r.away.is_empty() {
        if r.sent.take().is_some() {
            np.out.push(Msg::Away { who: Vec::new(), left: 0, ask: false });
        }
        (r.waited, r.asking, r.chosen) = (0.0, None, None);
        return switched(&np.room, &r.hidden);
    }
    r.waited += dt;
    if r.asking.is_none() && r.waited >= WAIT {
        r.asking = Some(CHOOSE);
    }
    if let Some(left) = &mut r.asking {
        *left -= dt;
        if let Some(bot) = press {
            r.chosen.get_or_insert(bot);
        }
        if *left <= 0.0 {
            r.chosen.get_or_insert(false);
        }
    }
    let left = r.asking.unwrap_or(WAIT - r.waited).ceil().max(0.0) as u32;
    let ask = r.asking.is_some();
    if r.sent != Some((left, ask)) {
        r.sent = Some((left, ask));
        np.out.push(Msg::Away { who: r.away.clone(), left, ask });
    }
    let who = names(&np.room, &r.away);
    match r.chosen.take() {
        Some(false) => {
            let mut room = np.room.clone();
            room.back_to_lobby(&r.away);
            np.out.push(Msg::Lobby(room.clone()));
            r.lobby = Some(room);
            np.finish(format!("{who} didn't come back. Back to the lobby...\n{score}"), false);
            String::new()
        }
        Some(true) => {
            info!("online: a bot plays for {who}");
            let mut room = np.room.clone();
            for s in room.seats.iter_mut().flatten().filter(|s| r.away.contains(&s.peer)) {
                s.peer = CPU.into();
            }
            r.ready = Some((0, u32::MAX, room, [[0; 6]; 4], Vec::new()));
            String::new()
        }
        None => {
            r.ask_me = ask;
            notice(&who, left, ask, false)
        }
    }
}

/// B162: who switched away, and how long until the wait for them runs out (empty: nobody).
fn switched(room: &Room, hidden: &BTreeMap<String, f32>) -> String {
    let Some(most) = hidden.values().copied().reduce(f32::max) else { return String::new() };
    let who = hidden.keys().map(|p| name(room, p)).collect::<Vec<_>>().join(", ");
    let left = (LONG - most).ceil().max(0.0) as u32;
    format!("{who} switched away from the game.\nWaiting for them to come back... {}:{:02}", left / 60, left % 60)
}

/// B162: each frame, this machine's keeper (made once the match has a network) hears it ran, and whether the window
/// had the focus (a minimised one loses it before its frames stop).
pub(super) fn keep(w: &mut World) {
    let focused = w.query_filtered::<&bevy::window::Window, With<bevy::window::PrimaryWindow>>().iter(w).next().is_none_or(|w| w.focused);
    let Some(np) = w.get_resource::<Netplay>() else { return };
    if np.recover.keeper.is_none() {
        let me = np.me.clone();
        let Some(k) = w.get_resource::<crate::net::Net>().map(|n| n.keeper(me)) else { return };
        w.resource_mut::<Netplay>().recover.keeper = Some(k);
    }
    if let Some(k) = &w.resource::<Netplay>().recover.keeper {
        k.frame(focused);
    }
}

fn notice(who: &str, left: u32, ask: bool, answered: bool) -> String {
    match (ask, answered) {
        // M126: up to WAIT's two minutes (a page reloading rejoins by itself)
        (false, _) => format!("Waiting for {who} to come back or rejoin... {}:{:02}", left / 60, left % 60),
        // the choices are the window's pills (`netplay.rs` `CHOICES`)
        (true, false) => format!("{who} didn't come back. ({left} s)"),
        (true, true) => format!("{who} didn't come back.\nWaiting for the others' choice... ({left} s)"),
    }
}

/// A restart ready: the host's goes out first (its state now), then every machine takes it on.
pub(super) fn restarted(w: &mut World) {
    let Some((round, tick, room, last, data)) = w.get_resource_mut::<Netplay>().and_then(|mut np| np.recover.ready.take()) else { return };
    let (round, tick, last, data) = if tick == u32::MAX {
        // the host's own: its state at a tick every seat's pads are in for
        let np = w.resource::<Netplay>();
        let (round, lockstep, now) = (np.recover.round.wrapping_add(1), np.room.lockstep, np.lock.tick);
        let state = if lockstep { Some((now, snapshot::encode(w))) } else { super::rollback::confirmed(w) };
        let Some((tick, data)) = state else {
            // ponytail: no confirmed cell this frame (it comes with the next gate): try again then
            w.resource_mut::<Netplay>().recover.ready = Some((0, u32::MAX, room, last, data));
            return;
        };
        let np = w.resource::<Netplay>();
        let last = tick.checked_sub(1).and_then(|t| np.lock.log.get(t as usize)).map_or([IDLE; 4], |p| *p).map(super::netplay::wire);
        let mut np = w.resource_mut::<Netplay>();
        let n = data.len().div_ceil(super::resync::PIECE).max(1);
        for (i, c) in data.chunks(super::resync::PIECE).enumerate() {
            np.out.push(Msg::Restart { round, tick, room: room.clone(), last, data: c.to_vec(), end: i + 1 == n });
        }
        (round, tick, last, data)
    } else {
        (round, tick, last, data)
    };
    if let Err(e) = snapshot::decode(w, &data) {
        warn!("online: the host's state didn't load: {e}");
        return w.resource_mut::<Netplay>().finish(format!("The match couldn't go on ({e})."), true);
    }
    let human: [bool; 4] = std::array::from_fn(|i| room.seats.get(i).is_some_and(|s| s.as_ref().is_some_and(Seat::human)));
    if let Some(mut p) = w.get_resource_mut::<Pads>() {
        p.seats = Some(human);
    }
    let mut np = w.resource_mut::<Netplay>();
    let np = &mut *np;
    info!("online: the match goes on from tick {tick} (restart {round}), people in seats {human:?}");
    let mine = (0..room.seats.len()).filter(|&i| room.seats[i].as_ref().is_some_and(|s| s.peer == np.me)).collect();
    np.lock = Lockstep::from(human, mine, np.lock.delay, tick, std::mem::take(&mut np.lock.log), last.map(unwire));
    np.intro = None;
    np.room = Room { rejoin: false, ..room };
    np.resync = super::resync::Resync { step: np.resync.step, ..Default::default() };
    (np.desynced, np.stalled) = (false, 0.0);
    let r = &mut np.recover;
    r.away.retain(|p| np.room.seats.iter().flatten().any(|s| s.peer == *p));
    for (p, q) in &mut r.quiet {
        if !r.away.contains(p) {
            q.0 = 0.0;
        }
    }
    (r.round, r.hold, r.told, r.sent, r.answered) = (round, false, None, None, false);
    if r.away.is_empty() {
        (r.waited, r.asking, r.chosen) = (0.0, None, None);
    }
    let (room, me, lockstep) = (np.room.clone(), np.me.clone(), np.room.lockstep);
    if !lockstep {
        super::rollback::restart(w, &room, &me, tick, round);
    }
}

#[cfg(test)]
mod tests {
    /// M126: the wait for an away player counts down in minutes and says it can rejoin.
    #[test]
    fn the_wait_says_minutes() {
        assert_eq!(super::notice("Sam", 120, false, false), "Waiting for Sam to come back or rejoin... 2:00");
        assert_eq!(super::notice("Sam", 9, false, false), "Waiting for Sam to come back or rejoin... 0:09");
    }
}
