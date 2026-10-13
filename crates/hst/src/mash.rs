//! `HST_MASH=secs[,seed[,frames]]`: a crash hunt — random menu keys (arrows, confirm, back, page, tab) pressed every
//! `frames` frames (default 2) from a seeded generator, then a normal quit after `secs`; the menu's own Quit is ignored until then. Off without the variable.

use bevy::prelude::*;

const KEYS: [KeyCode; 12] = [
    KeyCode::ArrowUp, KeyCode::ArrowDown, KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::ArrowLeft, KeyCode::ArrowRight,
    KeyCode::Enter, KeyCode::Enter, KeyCode::Escape, KeyCode::KeyQ, KeyCode::KeyE, KeyCode::Tab,
];

#[derive(Resource)]
struct Mash { until: f32, every: u32, seed: u64, frame: u32, held: Option<KeyCode> }

pub fn plugin(app: &mut App) {
    let Ok(v) = std::env::var("HST_MASH") else { return };
    let v: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    let mash = Mash { until: v.first().copied().unwrap_or(60.0), seed: v.get(1).map_or(1, |&s| s as u64 | 1), every: v.get(2).map_or(2, |&f| f.max(1.0) as u32), frame: 0, held: None };
    app.insert_resource(mash)
        .add_systems(PreUpdate, press.after(bevy::input::InputSystems))
        .add_systems(Last, stay.before(bevy::window::ExitSystems));
}

fn stay(m: Res<Mash>, time: Res<Time>, mut exit: ResMut<Messages<AppExit>>) {
    if time.elapsed_secs() < m.until {
        exit.clear();
    }
}

fn press(mut commands: Commands, mut m: ResMut<Mash>, mut keys: ResMut<ButtonInput<KeyCode>>, time: Res<Time>, mut exit: MessageWriter<AppExit>) {
    if let Some(k) = m.held.take() {
        keys.release(k);
    }
    if time.elapsed_secs() >= m.until {
        info!("HST_MASH: done, quitting");
        commands.insert_resource(crate::mode::Quit);
        exit.write(AppExit::Success);
        return;
    }
    m.frame += 1;
    if m.frame % m.every != 0 {
        return;
    }
    // xorshift64
    m.seed ^= m.seed << 13;
    m.seed ^= m.seed >> 7;
    m.seed ^= m.seed << 17;
    let k = KEYS[(m.seed % KEYS.len() as u64) as usize];
    keys.press(k);
    m.held = Some(k);
}

