//! The slide between menu screens (B40e2): the last screen's panels slide off and fade, then the new screen's slide
//! in from the other side. Ours, not the original's (menus need not be 1:1, user 2026-10-12): the wall and title bar
//! (`Anchor::Span`) switch at the midpoint without moving; a forward step (a later `Screen`) moves left, back moves right.

use bevy::prelude::*;

use super::super::widescreen::Anchor;
use super::{Quad, Screen};

/// Frames (60 Hz) for each half: out, then in.
const HALF: f32 = 8.0;
/// How far the panels travel (640×448 units) while fading.
const TRAVEL: f32 = 160.0;

#[derive(Resource)]
pub(super) struct Slide {
    on: Option<Screen>,
    /// The last frame's quads, and those of the screen being left.
    last: Vec<Quad>,
    old: Vec<Quad>,
    /// Frames since the screen changed.
    t: f32,
    /// +1 forward, −1 back.
    dir: f32,
}

impl Default for Slide {
    fn default() -> Self {
        Slide { on: None, last: Vec::new(), old: Vec::new(), t: 2.0 * HALF, dir: 1.0 }
    }
}

impl Slide {
    /// Feeds this frame's screen and quads (`dt` in 60 Hz frames); returns what to draw.
    pub(super) fn step(&mut self, screen: Screen, quads: Vec<Quad>, dt: f32) -> Vec<Quad> {
        if let Some(from) = self.on.filter(|&s| s != screen) {
            // no slide into a match (the window hides) or out of one (back to the menus)
            let still = matches!(screen, Screen::Playing) || matches!(from, Screen::Playing);
            self.t = if still { 2.0 * HALF } else { 0.0 };
            self.dir = if screen as u8 > from as u8 { 1.0 } else { -1.0 };
            self.old = std::mem::take(&mut self.last);
        } else {
            self.t += dt;
        }
        self.on = Some(screen);
        self.last = quads.clone();
        match self.incoming() {
            Some((dx, a)) => shift(quads, dx, a),
            None => {
                let e = ease(self.t / HALF);
                shift(self.old.clone(), -self.dir * TRAVEL * e, 1.0 - e)
            }
        }
    }

    /// The new screen's x shift and alpha factor, or `None` while the old one is still leaving.
    pub(super) fn incoming(&self) -> Option<(f32, f32)> {
        (self.t >= HALF).then(|| {
            let e = ease((self.t - HALF) / HALF);
            (self.dir * TRAVEL * (1.0 - e), e)
        })
    }
}

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn shift(mut quads: Vec<Quad>, dx: f32, a: f32) -> Vec<Quad> {
    for q in quads.iter_mut().filter(|q| q.anchor != Anchor::Span) {
        q.dst[0] += dx;
        q.alpha *= a;
    }
    quads
}

#[cfg(test)]
mod tests {
    use super::super::Tex;
    use super::*;

    fn quad(anchor: Anchor) -> Quad {
        Quad { tex: Tex::Solid, src: [0.0; 4], dst: [100.0, 0.0, 10.0, 10.0], rgb: [128.0; 3], alpha: 128.0, tile: false, anchor, angle: 0.0, round: [0.0; 4] }
    }

    #[test]
    fn slides_out_then_in() {
        let mut s = Slide::default();
        let main = vec![quad(Anchor::Centre), quad(Anchor::Span)];
        let mut mode = vec![quad(Anchor::Centre)];
        mode[0].dst[1] = 50.0;
        // first frame: nothing to slide from
        assert_eq!(s.step(Screen::Main, main.clone(), 1.0), main);
        // forward: Main's panel leaves to the left, fading; its wall stays
        let out = s.step(Screen::Mode, mode.clone(), 1.0);
        assert_eq!((out[0].dst[1], out[1].dst[0], out[1].alpha), (0.0, 100.0, 128.0));
        let mid = (1..=8).map(|_| s.step(Screen::Mode, mode.clone(), 1.0)).last().unwrap();
        assert!(mid[0].dst[1] == 50.0 && mid[0].dst[0] == 100.0 + TRAVEL && mid[0].alpha == 0.0);
        let half = (0..4).map(|_| s.step(Screen::Mode, mode.clone(), 1.0)).last().unwrap();
        assert!(half[0].dst[0] > 100.0 && half[0].dst[0] < 100.0 + TRAVEL);
        let done = (0..4).map(|_| s.step(Screen::Mode, mode.clone(), 1.0)).last().unwrap();
        assert_eq!(done, mode);
        // back: Mode's panel leaves to the right
        s.step(Screen::Main, main.clone(), 1.0);
        assert!(s.step(Screen::Main, main.clone(), 4.0)[0].dst[0] > 100.0);
        // into a match: no slide
        assert_eq!(s.step(Screen::Playing, main.clone(), 1.0), main);
    }
}
