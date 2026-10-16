//! The original's screen transition at the match's end: a still of the last frame drawn over the screen, fading out
//! (alpha counter/length, the counter starting at length + 2 and dropping one a tick) while the new scene runs under
//! it. 16 ticks from the match into the winners' ceremony, 60 from the ceremony into the result page (seen in the
//! original: `research/b38_transition.py`, `b38_hold.py`).
//!
//! The still is a screenshot of the window; the caller waits for it (10 frames at most: no window, no renderer)
//! before it changes the scene, as `mode::leave`.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};

/// The match → ceremony and ceremony → result page lengths.
pub(super) const TO_CEREMONY: i32 = 16;
pub(super) const TO_PAGE: i32 = 60;

#[derive(Resource, Default)]
pub(super) struct Dissolve {
    state: State,
    /// Frames waited for the capture.
    waited: u32,
    t: i32,
    len: i32,
    acc: f32,
}

#[derive(Default)]
enum State {
    #[default]
    Idle,
    Waiting,
    Captured(Handle<Image>),
    Fading(Handle<Image>),
}

#[derive(Component)]
struct Still;

impl Dissolve {
    /// Asks for the frame on screen; true once it is held (or can't be had), when the caller changes the scene and
    /// the still starts fading over `len` ticks.
    pub(super) fn cut(&mut self, commands: &mut Commands, len: i32) -> bool {
        match std::mem::take(&mut self.state) {
            State::Captured(h) => {
                (self.state, self.t, self.len, self.acc) = (State::Fading(h), len + 2, len, 0.0);
                true
            }
            State::Waiting if self.waited < 10 => {
                self.waited += 1;
                self.state = State::Waiting;
                false
            }
            State::Waiting => true,
            State::Idle | State::Fading(_) => {
                self.waited = 0;
                self.state = State::Waiting;
                commands.spawn(Screenshot::primary_window()).observe(captured);
                false
            }
        }
    }

    /// A still is fading.
    pub(super) fn fading(&self) -> bool {
        matches!(self.state, State::Fading(_))
    }

    /// The still's alpha this tick, if one is up.
    fn alpha(&self) -> Option<f32> {
        self.fading().then(|| (self.t as f32 / self.len as f32).min(1.0))
    }
}

fn captured(ev: On<ScreenshotCaptured>, mut d: ResMut<Dissolve>, mut images: ResMut<Assets<Image>>) {
    if matches!(d.state, State::Waiting) {
        // the capture comes back CPU-only: upload it to draw it
        let mut image = ev.image.clone();
        image.asset_usage = bevy::asset::RenderAssetUsages::default();
        d.state = State::Captured(images.add(image));
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Dissolve>().add_systems(Startup, setup).add_systems(Update, draw.after(super::step));
}

fn setup(mut commands: Commands) {
    let node = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    // over the match's overlays, under the widescreen bars and the loading cover
    commands.spawn((Still, node, ImageNode::default(), Visibility::Hidden, GlobalZIndex(60)));
}

/// Counts the still down at 60 Hz and draws it.
fn draw(time: Res<Time<Real>>, mut d: ResMut<Dissolve>, mut q: Query<(&mut ImageNode, &mut Visibility), With<Still>>) {
    if d.alpha().is_some() {
        d.acc += time.delta_secs() * 60.0;
        while d.acc >= 1.0 && d.t > 0 {
            d.acc -= 1.0;
            d.t -= 1;
        }
        if d.t <= 0 {
            d.state = State::Idle;
        }
    }
    let Ok((mut img, mut vis)) = q.single_mut() else { return };
    match (&d.state, d.alpha()) {
        (State::Fading(h), Some(a)) => {
            img.image = h.clone();
            img.color = Color::WHITE.with_alpha(a);
            *vis = Visibility::Visible;
        }
        _ => *vis = Visibility::Hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The counter starts at length + 2 (alpha clamped to 1 for the first ticks) and the still goes with it at 0.
    #[test]
    fn counts_down() {
        let mut d = Dissolve { state: State::Fading(Handle::default()), t: TO_PAGE + 2, len: TO_PAGE, ..default() };
        assert_eq!(d.alpha(), Some(1.0));
        d.t = 30;
        assert_eq!(d.alpha(), Some(0.5));
        d.state = State::Idle;
        assert_eq!(d.alpha(), None);
    }
}
