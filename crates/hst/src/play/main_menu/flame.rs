//! The menus' flame (`hst_sim::rng::MenuFlame`): made at P1's pick on the character select, behind the VS of the
//! confirmation screen, stepping at 60 Hz until the menus hand over to the match. Its `rand()` draws are the only
//! ones the original's menus make, so the match seed follows on from them (`rand`).

use bevy::prelude::*;
use hst_sim::rng::{MenuFlame, Rand};
use std::sync::Mutex;

use super::{Draw, Menu, Screen, Tex};

/// `f_s.tm2`: fire on the left half, smoke on the right.
const SHEET: Tex = Tex::Sheet(16);
/// The effect's origin on the 640×448 screen: the screen centre plus the original's node offset (-24, -136) and
/// the effect's own (32, 64).
const AT: [f32; 2] = [320.0 + 8.0, 224.0 - 72.0];
/// Frames from the setup until it steps (the original's start "~1 s" after the pick, from its logs).
const DELAY: u32 = 60;

static RAND: Mutex<Option<Rand>> = Mutex::new(None);

/// `rand()` as the menus left it, once they've run (`None`: the match was started without them).
pub fn rand() -> Option<Rand> {
    *RAND.lock().unwrap()
}

#[derive(Clone, Debug, Default)]
pub(super) struct Flame {
    rand: Rand,
    fx: Option<(MenuFlame, u32)>,
    picked: bool,
    acc: f32,
}

/// Made when P1 locks in on the select (again after backing out), stepped while the select or confirmation is up.
pub(super) fn tick(mut menu: ResMut<Menu>, time: Res<Time>) {
    let m = &mut *menu;
    let on = matches!(m.screen, Screen::Chars | Screen::Confirm);
    let f = &mut m.flame;
    let picked = on && (m.players[0].ready || m.screen == Screen::Confirm);
    if picked && !f.picked {
        f.fx = Some((MenuFlame::new(&mut f.rand), 0));
    }
    f.picked = picked;
    if !on {
        f.fx = None;
    }
    f.acc += time.delta_secs() * 60.0;
    while f.acc >= 1.0 {
        f.acc -= 1.0;
        if let Some((fx, n)) = &mut f.fx {
            *n += 1;
            if *n > DELAY {
                fx.frame(&mut f.rand);
            }
        }
    }
    *RAND.lock().unwrap() = Some(f.rand);
}

/// The particles about `AT`, under the VS.
pub(super) fn draw(d: &mut Draw, m: &Menu) {
    let Some((fx, _)) = &m.flame.fx else { return };
    for s in fx.sprites() {
        let src = [if s.smoke { 64.0 } else { 0.0 }, 0.0, 64.0, 64.0];
        let [x, y] = [AT[0] + s.at[0] - s.size / 2.0, AT[1] + s.at[1] - s.size / 2.0];
        d.q(SHEET, src, [x, y, s.size, s.size], super::WHITE, s.alpha);
        if let Some(q) = d.out.last_mut() {
            q.angle = s.angle;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rises_and_fades() {
        let mut r = Rand::default();
        let mut fx = MenuFlame::new(&mut r);
        let mut top = 0.0f32;
        for _ in 0..200 {
            fx.frame(&mut r);
            for s in fx.sprites() {
                assert!((0.0..=128.0).contains(&s.alpha) && s.size <= 64.0 * 1.56 + 0.01);
                top = top.min(s.at[1]);
            }
        }
        // it burns upward
        assert!(top < -40.0, "{top}");
    }
}
