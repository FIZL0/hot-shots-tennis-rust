//! M108: the match's side of the app going to the background (`android.rs` watches the activity). Offline, the pause
//! screen is up when the player comes back, so the match doesn't play on before they've looked; online, the match
//! a background leave left ends here as any leave does.

use super::netplay::Netplay;
use super::{Game, menu};
use bevy::prelude::*;

/// A `Cmd::Leave` for this online match's connection while it's still on, to send from another thread once the app
/// has been away too long; `None` offline or once the match has ended.
pub fn leaver(w: &World) -> Option<impl Fn() + Send + 'static> {
    let np = w.get_resource::<Netplay>()?;
    np.end.is_none().then_some(())?;
    Some(w.get_resource::<crate::net::Net>()?.leaver())
}

/// The app is back after a pause: online, `left` says the connection was dropped while it was away (the match ends,
/// as the peers have already seen it end); offline, the pause screen opens (not on the result screen, which has none).
pub fn back(w: &mut World, left: bool) {
    if let Some(mut np) = w.get_resource_mut::<Netplay>() {
        if left && np.end.is_none() {
            info!("online match over: left in the background");
            np.end = Some("You left the match (the app went to the background).".into());
        }
        return;
    }
    if w.get_resource::<Game>().is_some_and(|g| g.match_stats.result.is_none()) {
        menu::set_paused(w, true);
    }
}

/// M108, each frame (`First`, before the match's ticks): keeps `android::LEAVE` up to date, and on the first frame after a
/// pause, tells the match ([`back`]). Here, not in `android.rs`, as it
/// reaches into the match (P60).
#[cfg(target_os = "android")]
pub fn background(w: &mut World, mut seen: Local<u32>) {
    use crate::android::{LEAVE, LEFT, PAUSED, PAUSES};
    use std::sync::atomic::Ordering;
    let pauses = PAUSES.load(Ordering::Relaxed);
    if PAUSED.load(Ordering::Relaxed) {
        return;
    }
    if *seen != pauses {
        *seen = pauses;
        back(w, LEFT.swap(false, Ordering::Relaxed));
    }
    let leave = leaver(w);
    let mut l = LEAVE.lock().unwrap();
    match leave {
        Some(f) if l.is_none() => *l = Some(Box::new(f)),
        None => *l = None,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::{Cmd, Net, Room};

    #[test]
    fn an_online_match_left_in_the_background_ends() {
        let mut w = World::new();
        assert!(leaver(&w).is_none(), "offline: nothing to leave");
        back(&mut w, false);
        let (net, mut rx) = Net::wired();
        w.insert_resource(net);
        w.insert_resource(Netplay::new(Room::default(), "g".into()));
        let leave = leaver(&w).expect("an online match on");
        // a quick switch: back within the grace, the match goes on
        back(&mut w, false);
        assert!(w.resource::<Netplay>().end.is_none());
        leave();
        assert_eq!(rx.try_recv().ok(), Some(Cmd::Leave));
        back(&mut w, true);
        assert!(w.resource::<Netplay>().end.is_some(), "the match is over once the connection's gone");
        assert!(leaver(&w).is_none(), "nothing more to leave");
    }
}
