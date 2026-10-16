//! B162: a match's word to the others while this machine's frames have stopped because the player switched away (a
//! hidden browser tab, a window minimised or on another workspace): the frame loop sends nothing then, so a task of its
//! own (a thread natively, a timer on the page's event loop in the browser: about one a second while hidden) sends
//! `Msg::Hidden` every [`EVERY`]. The others hold the match ("… switched away", `play/recover.rs`) instead of taking
//! the player for gone. A frame loop stopped with the window in front (a hang) sends nothing: that's a silent peer.

use super::{Cmd, Msg, amp};
use n0_future::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// How often the keeper looks, and how long without a frame counts as stopped.
pub const EVERY: Duration = Duration::from_secs(1);

/// The match's handle on its keeper: stamp each frame; dropping it stops the keeper.
pub struct Keeper(Arc<Shared>);

struct Shared {
    start: Instant,
    /// The last frame (ms since `start`) and whether the window had the focus then.
    frame: AtomicU64,
    focused: AtomicBool,
    stop: AtomicBool,
}

impl Keeper {
    pub(super) fn start(cmd: amp::UnboundedSender<Cmd>, me: String, every: Duration) -> Keeper {
        let s = Arc::new(Shared { start: Instant::now(), frame: AtomicU64::new(0), focused: AtomicBool::new(true), stop: AtomicBool::new(false) });
        let k = s.clone();
        let step = move || {
            if k.stop.load(Ordering::Relaxed) {
                return false;
            }
            let since = k.start.elapsed().as_millis() as u64 - k.frame.load(Ordering::Relaxed);
            if since >= every.as_millis() as u64 && away(k.focused.load(Ordering::Relaxed)) {
                return cmd.send(Cmd::Send(Msg::Hidden(me.clone()))).is_ok();
            }
            true
        };
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::Builder::new()
            .name("net keeper".into())
            .spawn(move || {
                while {
                    std::thread::sleep(every);
                    step()
                } {}
            })
            .unwrap();
        #[cfg(target_arch = "wasm32")]
        n0_future::task::spawn(async move {
            while {
                n0_future::time::sleep(every).await;
                step()
            } {}
        });
        Keeper(s)
    }

    /// A frame ran; `focused`: the window has the focus.
    pub fn frame(&self, focused: bool) {
        self.0.frame.store(self.0.start.elapsed().as_millis() as u64, Ordering::Relaxed);
        self.0.focused.store(focused, Ordering::Relaxed);
    }
}

impl Drop for Keeper {
    fn drop(&mut self) {
        self.0.stop.store(true, Ordering::Relaxed);
    }
}

/// The player switched away: the window lost the focus at its last frame; in the browser also the page hidden now, or
/// its window not focused (a tab switch can stop the frames before the blur reaches the game).
fn away(focused: bool) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        let doc = js_sys::Reflect::get(&js_sys::global(), &"document".into()).ok();
        let hidden = doc.as_ref().and_then(|d| js_sys::Reflect::get(d, &"hidden".into()).ok()).is_some_and(|v| v.is_truthy());
        let focus = doc
            .as_ref()
            .and_then(|d| js_sys::Reflect::get(d, &"hasFocus".into()).ok().map(|f| (d.clone(), f)))
            .and_then(|(d, f)| wasm_bindgen::JsCast::dyn_ref::<js_sys::Function>(&f).and_then(|f| f.call0(&d).ok()))
            .is_none_or(|v| v.is_truthy());
        hidden || !focus || !focused
    }
    #[cfg(not(target_arch = "wasm32"))]
    !focused
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frames going on (focused or not), or stopped with the window in front: nothing; stopped unfocused: `Hidden`
    /// about every `every`, and nothing once dropped.
    #[test]
    fn speaks_only_for_a_player_switched_away() {
        let every = Duration::from_millis(50);
        let (tx, mut rx) = amp::unbounded_channel();
        let k = Keeper::start(tx, "g".into(), every);
        let heard = |rx: &mut amp::UnboundedReceiver<Cmd>| std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
        for i in 0..20 {
            k.frame(i % 2 == 0);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(heard(&mut rx), []);
        k.frame(true);
        std::thread::sleep(every * 5);
        assert_eq!(heard(&mut rx), []);
        k.frame(false);
        std::thread::sleep(every * 6);
        let said = heard(&mut rx);
        assert!((3..=6).contains(&said.len()), "{said:?}");
        assert!(said.iter().all(|c| *c == Cmd::Send(Msg::Hidden("g".into()))));
        drop(k);
        std::thread::sleep(every * 3);
        let _ = heard(&mut rx);
        std::thread::sleep(every * 3);
        assert_eq!(heard(&mut rx), []);
    }
}
