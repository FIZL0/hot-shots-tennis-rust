//! The original's pause screen (textures from `AZUMA/INPANE/INPANE.XB0`), coordinates on the PS2's 640×448 screen as
//! the panel. Start opens it (system sound 2) and freezes the match: the court dimmed (`i_pause_result_15` at alpha
//! 64), the top bar (`inpane_pause03`) with "Paused" and the rules ("1 sets 4 games", `inpane_pause02`), the score
//! board at the bottom (the result board's frame, plates and each set's games, with the points from
//! `inpane_pause00`, tiebreak `01`), and the dialog (`i_dialog_00`/`01`): "Resume the Match" / "Return to the Menu"
//! on pills, the chosen one bright, the hand (`hsm_yubi`) beside it swinging out 16 px and back.
//!
//! Input waits 5 ticks after it opens. Up/down move with wrap-around (repeat after 32 ticks held, then every 8;
//! sound 0, the hand starts its swing again), ✕ chooses (sound 2), ○ does nothing, Start closes it (sound 2).
//! "Return to the Menu" fades to black over 60 ticks; the port has no menu, so it then quits.
//! `HST_PAUSE=<seconds>` opens it after that long (for `--shot`).

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::sound;
use std::sync::Arc;
use std::time::Duration;

use super::panel::{self, Colours};
use super::{Game, Pads};
use crate::Args;
use crate::audio::{Sound, SoundBank};

const TEXTURES: [&str; 11] = [
    "/i_pause_result_15.tm2",
    "/inpane_pause00.tm2",
    "/inpane_pause01.tm2",
    "/inpane_pause02.tm2",
    "/inpane_pause03.tm2",
    "/result_gameset00.tm2",
    "/result_gameset01.tm2",
    "/result_gameset02.tm2",
    "/i_dialog_00.tm2",
    "/i_dialog_01.tm2",
    "/hsm_yubi.tm2",
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tex {
    /// `TEXTURES[k]`.
    Own(usize),
    /// The panel's slot labels.
    Slot,
    Face(usize),
}
const DIM: Tex = Tex::Own(0);
const POINTS: Tex = Tex::Own(1);
const TIEBREAK: Tex = Tex::Own(2);
const TEXT: Tex = Tex::Own(3);
const BAR: Tex = Tex::Own(4);
const BOARD: Tex = Tex::Own(5);
const GAMES_PAST: Tex = Tex::Own(6);
const GAMES_NOW: Tex = Tex::Own(7);
const FRAME: Tex = Tex::Own(8);
const ROWS: Tex = Tex::Own(9);
const HAND: Tex = Tex::Own(10);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Quad {
    tex: Tex,
    src: [f32; 4],
    dst: [f32; 4],
    rgb: [f32; 3],
    alpha: f32,
}

/// The dialog's text rows in `i_dialog_01`: "Resume the Match", "Return to the Menu".
const OPTIONS: [i32; 2] = [6, 1];
/// Points cells (column, row) by point index; 4 is Deuce, 5 Advantage (tiebreak: 0–7, 8 Deuce, 9 Advantage).
const POINT_U: [i32; 10] = [0, 1, 0, 1, 0, 1, 0, 1, 0, 1];
const POINT_V: [i32; 10] = [0, 0, 1, 1, 2, 2, 3, 3, 4, 4];
const WHITE: [f32; 3] = [128.0; 3];

/// The hand's swing: out to −16 slowing down, an 11-tick rest, then back speeding up (57 ticks a cycle).
#[derive(Clone, Copy, Default, Debug)]
struct Hand {
    x: f32,
    t: u32,
    stage: u32,
}

impl Hand {
    fn step(&mut self) {
        let t = self.t as f32;
        match self.stage {
            0 => {
                self.x += (t / 3.0).powi(2) / 100.0 + 0.35;
                if self.x >= 0.0 {
                    *self = Hand { x: 0.0, t: 0, stage: 1 };
                }
            }
            1 => {
                self.x -= ((30.0 - t) / 3.0).powi(2) / 100.0 + 0.35;
                if self.x <= -16.0 {
                    *self = Hand { x: -16.0, t: 0, stage: 2 };
                }
            }
            _ if self.t >= 10 => (self.stage, self.t) = (0, 0),
            _ => {}
        }
        self.t += 1;
    }
}

/// The pause state, stepped at 60 Hz.
#[derive(Resource, Default)]
struct Menu {
    open: bool,
    /// Ticks into the fade to black after "Return to the Menu".
    quitting: Option<i32>,
    /// Ticks left before input is read.
    delay: i32,
    sel: usize,
    hand: Hand,
    /// Ticks up/down held (for the repeat).
    held: [u32; 2],
    /// Presses since the last tick: Start, ✕.
    start: bool,
    confirm: bool,
    acc: f32,
}

impl Menu {
    fn opened() -> Self {
        Menu { open: true, delay: 5, ..default() }
    }

    /// One tick of the open menu with up/down held; returns the system sound to play.
    fn tick(&mut self, up: bool, down: bool) -> Option<usize> {
        if self.quitting.is_some() {
            self.quitting = self.quitting.map(|t| t + 1);
            return None;
        }
        if std::mem::take(&mut self.start) {
            *self = Menu::default();
            return Some(2);
        }
        let mut sound = None;
        if self.delay < 1 {
            // first press, then after 32 ticks held every 8; one direction at a time
            let repeat = |on: bool, n: &mut u32| {
                *n = if on { *n + 1 } else { 0 };
                *n == 1 || (*n >= 32 && (*n - 32) % 8 == 0)
            };
            let up = self.held[1] == 0 && repeat(up, &mut self.held[0]);
            let down = self.held[0] == 0 && repeat(down, &mut self.held[1]);
            let n = OPTIONS.len();
            if down || up {
                self.sel = if down { (self.sel + 1) % n } else { (self.sel + n - 1) % n };
                self.hand = Hand::default();
                sound = Some(0);
            } else if self.confirm {
                sound = Some(2);
                match self.sel {
                    0 => *self = Menu::default(),
                    _ => self.quitting = Some(0),
                }
            }
        } else {
            self.delay -= 1;
        }
        self.confirm = false;
        self.hand.step();
        sound
    }
}

/// What the pause screen shows.
struct View {
    players: usize,
    slots: [usize; 4],
    pill: [[u8; 3]; 4],
    /// Rules: sets to win, games a set.
    sets_to_win: i32,
    games: i32,
    points: [i32; 2],
    deuce: bool,
    advantage: bool,
    tiebreak: bool,
    /// The set in play (0-based) and each team's games per set.
    set: i32,
    set_games: [[i32; 5]; 2],
    sel: usize,
    hand_x: f32,
    fade: Option<i32>,
}

fn layout(v: &View) -> Vec<Quad> {
    let mut out = Vec::new();
    let mut q = |tex, src: [i32; 4], dst: [f32; 4], rgb: [f32; 3], alpha: i32| {
        out.push(Quad { tex, src: src.map(|c| c as f32), dst, rgb, alpha: alpha as f32 })
    };
    let r = |x: i32, y: i32, w: i32, h: i32| [x, y, w, h].map(|c| c as f32);
    let rgb = |c: [u8; 3]| c.map(f32::from);
    q(DIM, [0, 0, 8, 8], r(0, 0, 640, 448), WHITE, 64);
    // the top bar and the rules: best-of sets in red, games in yellow
    q(BAR, [0, 0, 16, 64], r(0, 0, 352, 64), WHITE, 128);
    q(BAR, [16, 0, 32, 64], r(352, 0, 32, 64), WHITE, 128);
    q(BAR, [48, 0, 16, 64], r(384, 0, 256, 64), WHITE, 128);
    q(TEXT, [0, 0, 64, 32], r(24, 16, 64, 32), WHITE, 128);
    q(TEXT, [(v.sets_to_win * 2 - 2) * 16, 40, 16, 24], r(176, 24, 16, 24), [127.0, 59.0, 59.0], 128);
    q(TEXT, [0, 64, 56, 24], r(192, 24, 56, 24), WHITE, 128);
    q(TEXT, [(v.games - 1) * 16, 40, 16, 24], r(248, 24, 16, 24), [127.0, 97.0, 24.0], 128);
    q(TEXT, [0, 88, 104, 24], r(264, 24, 104, 24), WHITE, 128);

    // the score board: the result board's frame and plates, each set's games, the points either side
    let short = (1..=2).contains(&v.sets_to_win);
    let (rows, edge_h, top, y0, centre_h, diag_y) =
        if short { (6, 80, 320, 328, 96, 368) } else { (10, 144, 256, 264, 160, 336) };
    let n = v.players;
    for p in 0..n {
        let (count, off) = if n < 3 { (rows, 0) } else { (rows / 2, if p > 1 { rows * 8 } else { 0 }) };
        for k in 0..count {
            q(BOARD, [1 + 16 * p as i32, 40, 14, 16], r([96, 456][p & 1], y0 + off + 16 * k, 88, 16), WHITE, 128);
        }
    }
    if n >= 3 {
        for side in 0..2 {
            q(BOARD, [0, [56, 80][side], 88, 24], r([96, 456][side], diag_y, 88, 24), WHITE, 128);
        }
    }
    let bottom = top + 16 + edge_h;
    for (u, x, w) in [(0, 88, 16), (16, 104, 432), (40, 536, 16)] {
        q(BOARD, [u, 0, 16, 16], r(x, top, w, 16), WHITE, 128);
        q(BOARD, [u, 16, 16, 8], r(x, top + 16, w, edge_h), WHITE, 128);
        q(BOARD, [u, 24, 16, 16], r(x, bottom, w, 16), WHITE, 128);
    }
    q(BOARD, [65, 41, 14, 14], r(184, y0, 272, centre_h), WHITE, 128);
    for x in [248, 390, 184, 454] {
        q(BOARD, [64, 9, 8, 22], r(x, y0, 8, centre_h), WHITE, 128);
    }
    let (y1, y2) = if short { (328, 380) } else { (272, 372) };
    let plate_y = if n < 3 { [y1 + (y2 - y1) / 2; 2] } else { [y1, y2] };
    for p in 0..n {
        let (x, y) = ([100, 496][p & 1], plate_y[p / 2]);
        q(BOARD, [80, 0, 48, 48], r(x, y, 48, 48), WHITE, 128);
        q(Tex::Face(p), [0, 0, 64, 64], r(x + 2, y + 2, 64, 64), WHITE, 128);
        q(Tex::Slot, [v.slots[p] as i32 * 40, 0, 40, 24], r([144, 456][p & 1], y + 6, 40, 24), rgb(v.pill[p]), 128);
    }
    // the set in play on the bright sheet, a past set the team didn't win at half alpha
    let (cols, games_y) = match v.sets_to_win {
        1 => (1, 360),
        2 => (3, 328),
        _ => (5, 264),
    };
    for team in 0..2 {
        for col in 0..cols {
            let y = games_y + 32 * col;
            let sheet = if col == v.set { GAMES_NOW } else { GAMES_PAST };
            if col <= v.set {
                let c = col.min(4) as usize;
                let g = v.set_games[team][c];
                let alpha = if col == v.set || v.set_games[team ^ 1][c] < g { 128 } else { 64 };
                q(sheet, [g * 32, 0, 32, 32], r([272, 336][team], y, 32, 32), WHITE, alpha);
            }
            if team == 0 {
                q(sheet, [320, 0, 24, 32], r(308, y, 24, 32), WHITE, 128);
            }
        }
    }
    // the points; with advantage the trailing team at half alpha
    let (tex, last) = if v.tiebreak { (TIEBREAK, 9) } else { (POINTS, 5) };
    for t in 0..2 {
        let p = v.points[t].clamp(0, last);
        let k = match (v.deuce, v.advantage) {
            (true, _) => last - 1,
            (false, true) if p >= last - 1 => last,
            _ => p,
        } as usize;
        let alpha = if v.advantage && k != last as usize { 64 } else { 128 };
        q(tex, [POINT_U[k] << 6, POINT_V[k] * 48, 64, 48], r([184, 390][t], if short { 352 } else { 320 }, 64, 48), WHITE, alpha);
    }

    // the dialog: frame, a pill per option (bright when chosen) under a white sheen, the text, the hand
    let nopt = OPTIONS.len() as i32;
    let lift = if v.sets_to_win == 3 { -72 } else { -32 };
    let top = lift - (nopt - 1) * 16 + 0xb8;
    let mut y = top;
    let mut row = |vv: i32, h: i32, y: i32| {
        q(FRAME, [0, vv, 16, h], r(160, y, 16, h), WHITE, 128);
        q(FRAME, [16, vv, 8, h], r(176, y, 288, h), WHITE, 128);
        q(FRAME, [24, vv, 16, h], r(464, y, 16, h), WHITE, 128);
    };
    row(0, 16, y);
    y += 16;
    for k in 0..nopt * 5 {
        row(16, 8, y + 8 * k);
    }
    y += 8 * (nopt * 5 - 1);
    row(24, 16, y + 8);
    let y0 = lift - (nopt - 1) * 16 + 0xcc;
    for (k, _) in OPTIONS.iter().enumerate() {
        let tint = if k == v.sel { [123.0, 105.0, 79.0] } else { [94.0, 83.0, 70.0] };
        let y = y0 + 40 * k as i32;
        for (vv, rgb) in [(48, tint), (88, WHITE)] {
            q(FRAME, [0, vv, 16, 32], r(176, y, 16, 32), rgb, 128);
            q(FRAME, [16, vv, 8, 32], r(192, y, 256, 32), rgb, 128);
            q(FRAME, [24, vv, 16, 32], r(448, y, 16, 32), rgb, 128);
        }
    }
    for (k, &text) in OPTIONS.iter().enumerate() {
        let (rgb, alpha) = if k == v.sel { (WHITE, 128) } else { ([94.0, 83.0, 70.0], 64) };
        q(ROWS, [0, text * 32, 256, 32], r(192, y0 + 40 * k as i32, 256, 32), rgb, alpha);
    }
    let hy = (y0 + v.sel as i32 * 40 + 8) as f32;
    out.push(Quad { tex: HAND, src: [0.0, 0.0, 64.0, 32.0], dst: [v.hand_x + 128.0, hy, 64.0, 32.0], rgb: WHITE, alpha: 128.0 });
    if let Some(t) = v.fade {
        let a = (128 * t / 60).min(128) as f32;
        out.push(Quad { tex: DIM, src: [0.0, 0.0, 8.0, 8.0], dst: [0.0, 0.0, 640.0, 448.0], rgb: [0.0; 3], alpha: a });
    }
    out
}

#[derive(Resource)]
struct Art(Vec<Handle<Image>>, Option<Arc<SoundBank>>);
#[derive(Component)]
struct Slot(usize);
#[derive(Component)]
struct Root;
const POOL: usize = 160;

pub fn plugin(app: &mut App) {
    app.init_resource::<Menu>()
        .add_systems(PostStartup, setup.after(super::setup))
        .add_systems(Update, (step, draw).chain());
}

fn setup(mut commands: Commands, args: Res<Args>, mut images: ResMut<Assets<Image>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso.read("AZUMA/INPANE/INPANE.XB0").expect("INPANE archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let art = TEXTURES
        .iter()
        .map(|name| {
            let e = arc
                .entries
                .iter()
                .find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name))
                .unwrap_or_else(|| panic!("{name} in INPANE"));
            panel::image(&mut images, &arc.read(e).expect("INPANE bytes"))
        })
        .collect();
    let bank = SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd").map(Arc::new);
    commands.insert_resource(Art(art, bank));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(10),
            Root,
        ))
        .with_children(|p| {
            for i in 0..POOL {
                p.spawn((
                    Slot(i),
                    ImageNode { image_mode: NodeImageMode::Stretch, ..default() },
                    Node { position_type: PositionType::Absolute, ..default() },
                    Visibility::Hidden,
                ));
            }
        });
}

/// Reads Start/✕/up/down, steps the menu at 60 Hz and holds the match's fixed clock while it is open.
fn step(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    mut fixed: ResMut<Time<Fixed>>,
    mut menu: ResMut<Menu>,
    mut pads: ResMut<Pads>,
    art: Option<Res<Art>>,
    sound: Option<Res<Sound>>,
    mut exit: MessageWriter<AppExit>,
    mut auto: Local<bool>,
) {
    let any = |b: GamepadButton| gamepads.iter().any(|g| g.just_pressed(b));
    let start = keys.just_pressed(KeyCode::Escape) || any(GamepadButton::Start);
    let confirm = keys.any_just_pressed([KeyCode::Enter, KeyCode::Space, KeyCode::KeyJ]) || any(GamepadButton::South);
    // the stick as the original's menus read it: its byte's held direction (|48| of 127, past the driver's deadzone)
    let stick = |g: &Gamepad| -hst_sim::player::pad_held(super::stick_byte(-g.left_stick().y)) as f32 + g.dpad().y;
    let up = keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) || gamepads.iter().any(|g| stick(g) > 0.5);
    let down = keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) || gamepads.iter().any(|g| stick(g) < -0.5);
    let play = |key: usize| {
        if let (Some(s), Some(Art(_, Some(b)))) = (&sound, art.as_deref()) {
            s.play_centre(b, sound::Play { slot: 9, program: 0, key: key as u8, volume: 0x80, speed: 1.0 });
        }
    };
    let timed = !*auto
        && std::env::var("HST_PAUSE").ok().and_then(|s| s.parse::<f32>().ok()).is_some_and(|t| time.elapsed_secs() >= t);
    if !menu.open {
        if start || timed {
            *auto = true;
            *menu = Menu::opened();
            play(2);
            // ponytail: the fixed clock runs on but never reaches a step; the overstep is thrown away on closing
            fixed.set_timestep(Duration::from_secs(1 << 20));
        }
        return;
    }
    menu.start |= start;
    menu.confirm |= confirm;
    menu.acc += time.delta_secs() * 60.0;
    while menu.acc >= 1.0 && menu.open {
        menu.acc -= 1.0;
        if let Some(k) = menu.tick(up, down) {
            play(k);
        }
    }
    // presses made in the menu never reach the match
    for s in &mut pads.slots {
        s.shot = None;
        s.serve = false;
    }
    if menu.quitting.is_some_and(|t| t >= 60) {
        exit.write(AppExit::Success);
    }
    if !menu.open {
        fixed.set_timestep_hz(60.0);
        let over = fixed.overstep();
        fixed.discard_overstep(over);
    }
}

fn draw(
    g: Res<Game>,
    pads: Res<Pads>,
    menu: Res<Menu>,
    art: Option<Res<Art>>,
    panel_art: Option<Res<panel::Art>>,
    colours: Option<Res<Colours>>,
    mut q: Query<(&Slot, &mut ImageNode, &mut Node, &mut Visibility)>,
    mut others: Query<(Entity, &mut Visibility), (With<Node>, Without<ChildOf>, Without<Root>, Without<Slot>)>,
    mut hidden: Local<Vec<(Entity, Visibility)>>,
) {
    let (Some(art), Some(panel_art), Some(colours)) = (art, panel_art, colours) else {
        return;
    };
    let n = g.players.len();
    let s = &g.score;
    // the pause screen replaces the rest of the HUD (panel, pop-ups, the port's text)
    if menu.open && hidden.is_empty() {
        for (e, mut vis) in &mut others {
            hidden.push((e, *vis));
            *vis = Visibility::Hidden;
        }
    } else if !menu.open {
        for (e, vis) in hidden.drain(..) {
            if let Ok((_, mut v)) = others.get_mut(e) {
                *v = vis;
            }
        }
    }
    let quads = if menu.open {
        layout(&View {
            players: n,
            slots: std::array::from_fn(|i| if i < n { pads.slot_of(i, n).unwrap_or(4) } else { 4 }),
            pill: colours.0,
            sets_to_win: g.rules.sets.clamp(1, 3),
            games: g.rules.games.clamp(1, 6),
            points: s.points,
            deuce: s.deuce,
            advantage: s.advantage,
            tiebreak: s.tiebreak,
            set: s.set,
            set_games: s.set_games,
            sel: menu.sel,
            hand_x: menu.hand.x,
            fade: menu.quitting,
        })
    } else {
        Vec::new()
    };
    for (Slot(i), mut img, mut node, mut vis) in &mut q {
        let Some(quad) = quads.get(*i) else {
            *vis = Visibility::Hidden;
            continue;
        };
        img.image = match quad.tex {
            Tex::Own(k) => art.0[k].clone(),
            Tex::Slot => panel_art.0[1].clone(),
            Tex::Face(p) => panel_art.0[panel::FACES + p].clone(),
        };
        let [u, v, w, h] = quad.src;
        img.rect = Some(Rect::new(u, v, u + w, v + h));
        let [r, g, b] = quad.rgb.map(|c| c / 128.0);
        img.color = Color::srgba(r, g, b, quad.alpha / 128.0);
        let [x, y, w, h] = quad.dst;
        node.left = Val::Percent(x / 6.4);
        node.top = Val::Percent(y / 4.48);
        node.width = Val::Percent(w / 6.4);
        node.height = Val::Percent(h / 4.48);
        *vis = Visibility::Inherited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> View {
        View {
            players: 4,
            slots: [0, 4, 4, 4],
            pill: [[98, 46, 61], [33, 74, 97], [105, 70, 25], [63, 78, 25]],
            sets_to_win: 1,
            games: 4,
            points: [0, 0],
            deuce: false,
            advantage: false,
            tiebreak: false,
            set: 0,
            set_games: [[0; 5]; 2],
            sel: 0,
            hand_x: 0.0,
            fade: None,
        }
    }

    /// Slot 3's pause (doubles, one set to 4 games, 0-0): the rules digits, both points, the games and the dialog.
    #[test]
    fn slot3_pause() {
        let qs = layout(&view());
        let at = |tex: Tex, x: f32, y: f32| qs.iter().find(|q| q.tex == tex && q.dst[0] == x && q.dst[1] == y).copied();
        assert_eq!(at(TEXT, 176.0, 24.0).unwrap().src, [0.0, 40.0, 16.0, 24.0]); // "1"
        assert_eq!(at(TEXT, 248.0, 24.0).unwrap().src, [48.0, 40.0, 16.0, 24.0]); // "4"
        assert_eq!(at(POINTS, 184.0, 352.0).unwrap().src, [0.0, 0.0, 64.0, 48.0]);
        assert!(at(POINTS, 390.0, 352.0).is_some());
        assert!(at(GAMES_NOW, 272.0, 360.0).is_some() && at(GAMES_NOW, 308.0, 360.0).is_some());
        assert!(at(Tex::Face(2), 102.0, 382.0).is_some() && at(Tex::Face(1), 498.0, 330.0).is_some());
        // frame from y 136 to 248, pills at 156 and 196, the hand beside the first
        assert!(at(FRAME, 160.0, 136.0).is_some() && at(FRAME, 160.0, 232.0).is_some());
        let rows: Vec<_> = qs.iter().filter(|q| q.tex == ROWS).collect();
        assert_eq!((rows[0].src[1], rows[0].dst[1], rows[0].alpha), (192.0, 156.0, 128.0));
        assert_eq!((rows[1].src[1], rows[1].dst[1], rows[1].alpha), (32.0, 196.0, 64.0));
        assert_eq!(qs.iter().find(|q| q.tex == HAND).unwrap().dst, [128.0, 164.0, 64.0, 32.0]);
    }

    /// Advantage: "A" for the leader, the trailing 40 at half alpha; tiebreak deuce uses the red D.
    #[test]
    fn points() {
        let qs = layout(&View { points: [4, 3], advantage: true, ..view() });
        let pts: Vec<_> = qs.iter().filter(|q| q.tex == POINTS).map(|q| (q.src, q.alpha)).collect();
        assert_eq!(pts, [([64.0, 96.0, 64.0, 48.0], 128.0), ([64.0, 48.0, 64.0, 48.0], 64.0)]);
        let qs = layout(&View { points: [6, 6], deuce: true, tiebreak: true, ..view() });
        assert!(qs.iter().filter(|q| q.tex == TIEBREAK).all(|q| q.src == [0.0, 192.0, 64.0, 48.0]));
    }

    /// Down moves and wraps, input waits 5 ticks, the hand swings to −16 and back, ✕ on the second option fades out.
    #[test]
    fn input() {
        let mut m = Menu::opened();
        for _ in 0..5 {
            assert_eq!(m.tick(false, true), None);
        }
        assert_eq!(m.tick(false, true), Some(0));
        assert_eq!(m.sel, 1);
        assert_eq!(m.tick(false, true), None); // held: no repeat until 32
        for _ in 0..29 {
            m.tick(false, true);
        }
        assert_eq!(m.tick(false, true), Some(0)); // 32nd tick held
        assert_eq!(m.sel, 0);
        let mut h = Hand::default();
        let xs: Vec<f32> = (0..60).map(|_| { h.step(); h.x }).collect();
        // out to −16 by tick 20, held 11 ticks, eased back to 0 at tick 58 and out again
        assert!(xs[19] > -16.0 && xs[20] == -16.0 && xs[30] == -16.0 && xs[31] > -16.0);
        assert!(xs[56] < 0.0 && xs[57] == 0.0 && xs[58] < 0.0);
        m.tick(false, false);
        m.tick(true, false);
        assert_eq!(m.sel, 1);
        m.confirm = true;
        assert_eq!(m.tick(false, false), Some(2));
        assert_eq!(m.quitting, Some(0));
        m.start = true;
        m.tick(false, false);
        assert!(m.open && m.quitting == Some(1));
    }
}
