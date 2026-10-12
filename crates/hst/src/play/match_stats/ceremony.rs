//! The winners' ceremony before the result page (the original's first result state): the shown side's camera clip
//! (`AZUMA/C_EFF/EFFCT.XB0` `prize/CAM/win_99`, `lost_99` or `hand_99`), every player in its ceremony motion on its
//! mark (`AZUMA/PRIZE/ANI/WINANI.XB`, `LOSTANI.XB`), confetti for a winner, the "NARROW WIN" etc. banner from
//! `result_gameset05` and in doubles the team's name plate (`inpane_team`), and the match-end sounds: the shown
//! character's prize voice (a bit of the sound generator), the gallery's ovation or groan and, for a winner, four
//! cheers and the BGM fading out over 270 ticks. It runs for the camera clip's length, then the result page opens;
//! the figures and the clip's last view stay behind the result page. The sim and its generators are never touched:
//! the ending is copied in `match_over` and the voice's bit drawn from that copy.

use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use hst_data::{ani, iso::Iso, xb::Archive};
use hst_sim::pose::{Clip, q_matrix};
use hst_sim::rng::Mt;
use hst_sim::sound::{self, Gallery, Reaction};

use super::{Quad, Tex, WHITE};
use crate::audio::{Sound, SoundBank};
use crate::character::Rig;

/// What the match's end leaves for the ceremony (`match_over`, before the next match resets the score and reseeds
/// the sound generator): the ends as they stood, and the sound generator.
/// ponytail: a static so play.rs's `Game` keeps its fields; one match ends at a time
pub(super) static ENDING: Mutex<Option<(bool, Mt)>> = Mutex::new(None);

/// The players' marks: (x, y, z) for the near side, singles and doubles.
const MARK: [[f32; 3]; 2] = [[0.0, 0.0, -6.4], [0.75, 0.0, -6.4]];
/// Per character: the hand slap's file (`hand_<XX><l|r><suffix>`), and whether its lost motion's first pair is `b`.
const HAND: [(&str, &str); 14] = [
    ("00", ""), ("00", "_m"), ("00", "_b"), ("03", ""), ("00", ""), ("01", ""), ("00", "_n"),
    ("02", ""), ("00", ""), ("00", "_m"), ("04", ""), ("05", "_n"), ("00", ""), ("06", ""),
];
const LOST_B: [bool; 14] = [false, false, true, false, true, true, false, true, true, false, false, true, true, false];
/// The four cheers for a winner: bearings and volume (gallery program 10 key 1).
const CHEERS: [i32; 4] = [45, 135, 225, 325];
const CHEER_VOLUME: i32 = 0x20;
/// The BGM's level and its fade-out's length.
const BGM: f32 = 55.0;
const FADE: u32 = 270;

/// Motion slots.
const WIN: usize = 0;
const HANDS: usize = 1;
const LOST: usize = 2;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Plan {
    /// The side the camera shows, and whether it lost (a human side beaten by the bots).
    pub shown: usize,
    pub lost: bool,
    /// Camera clip (0 win, 1 lost, 2 the doubles hand slap) and whether it is turned π about y (the far end).
    pub clip: usize,
    pub turned: bool,
    pub offset: [f32; 3],
    /// `result_gameset05` row: 0/1 annihilation, 2/3 victory/defeat, 4/5 narrow win/loss.
    pub row: usize,
    /// Each player's mark, whether it faces −z, its motion files (win, hand, lost) and the slot it starts in.
    pub marks: Vec<([f32; 3], bool)>,
    pub files: Vec<[String; 3]>,
    pub start: Vec<usize>,
    /// The voice and the gallery's reaction at once (singles, or a beaten side), not at the hand slap's end.
    pub voice_now: bool,
}

/// The ceremony for `humans` (players on a controller), `chars`, the `winner` side, the ends `swapped` and the
/// winner's rating (stats row 8).
pub(super) fn plan(humans: &[bool], chars: &[i32], winner: usize, swapped: bool, rating: f32) -> Plan {
    let n = humans.len();
    let doubles = n > 2;
    let mut human = [false; 2];
    for (i, &h) in humans.iter().enumerate() {
        human[i & 1] |= h;
    }
    let one = human[0] != human[1];
    let shown = if one { human[1] as usize } else { winner };
    let lost = shown != winner;
    let mark = MARK[doubles as usize];
    let marks: Vec<_> = (0..n)
        .map(|i| {
            // the near side as the mark, the far one turned with z mirrored; one of each pair mirrored in x
            let far = (i & 1 == 1) != swapped;
            let mut p = mark;
            if far {
                p[2] = -p[2];
            }
            let flip = if swapped { [0, 3] } else { [2, 1] };
            if doubles && flip.contains(&i) {
                p[0] = -p[0];
            }
            (p, far)
        })
        .collect();
    let mut offset = [0.0, 0.0, marks[shown].0[2]];
    if lost {
        offset[1] = -1.3;
    } else if !one && doubles && [chars[winner], chars[winner + 2]].iter().any(|&c| c == 7 || c == 11) {
        offset[1] -= 0.4;
    }
    let files = (0..n)
        .map(|i| {
            let c = chars[i].clamp(0, 13) as usize;
            let n_ = if c == 6 || c == 11 { "_n" } else { "" };
            let (xx, suffix) = HAND[c];
            let lr = if i < 2 { 'l' } else { 'r' };
            let ab = if LOST_B[c] != (i >= 2) { 'b' } else { 'a' };
            [format!("win/win_{c:02}{n_}"), format!("hand/hand_{xx}{lr}{suffix}"), format!("lost/lost_{c:02}{ab}{n_}")]
        })
        .collect();
    let start = (0..n).map(|i| if i & 1 != winner { LOST } else if doubles { HANDS } else { WIN }).collect();
    let base = if rating < 65.0 { 4 } else if rating < 80.0 { 2 } else { 0 };
    Plan {
        shown,
        lost,
        clip: if lost { 1 } else if doubles { 2 } else { 0 },
        turned: (shown == 0) == swapped,
        offset,
        row: base + lost as usize,
        marks,
        files,
        start,
        voice_now: lost || !doubles,
    }
}

/// The banner and, in doubles, the shown team's name plate (640×448).
pub(super) fn quads(plan: &Plan, doubles: bool) -> Vec<Quad> {
    let q = |tex, src: [i32; 4], dst: [i32; 4], rgb| Quad { tex, src: src.map(|c| c as f32), dst: dst.map(|c| c as f32), rgb, alpha: 128.0 };
    let mut out = vec![q(Tex::Gameset, [0, plan.row as i32 * 80, 256, 80], [192, 360, 256, 80], WHITE)];
    if doubles {
        let tint = if plan.shown == 0 { [127.0, 77.0, 77.0] } else { [67.0, 101.0, 127.0] };
        for (v, rgb) in [(96, tint), (64, WHITE)] {
            out.push(q(Tex::Team, [0, v, 16, 32], [240, 320, 16, 32], rgb));
            out.push(q(Tex::Team, [16, v, 16, 32], [256, 320, 128, 32], rgb));
            out.push(q(Tex::Team, [32, v, 16, 32], [384, 320, 16, 32], rgb));
        }
        out.push(q(Tex::Team, [0, if plan.shown == 0 { 0 } else { 32 }, 96, 32], [272, 320, 96, 32], WHITE));
    }
    out
}

/// One confetti piece, in the camera's frame (Bevy axes: x right, y up, −z ahead).
struct Flake {
    at: Vec3,
    axis: Vec3,
    spin: f32,
    phase: f32,
    entity: Entity,
}

/// The confetti's box around the point 2.4 ahead of the camera, its fall and sway per tick, its count and size.
const BOX: Vec3 = Vec3::new(1.7, 1.7, 1.5);
const AHEAD: f32 = 2.4;
const FALL: f32 = 0.015;
const SWAY: f32 = 0.02;
const FLAKES: usize = 200;
const FLAKE: f32 = 0.04;

#[derive(Resource)]
pub(super) struct Ceremony {
    pub plan: Plan,
    /// Ticks run, and the camera clip's length (frames).
    pub t: u32,
    length: u32,
    /// The camera clip's keys, one per frame: rotation, position, full field of view.
    cam: Vec<([f32; 4], [f32; 4], f32)>,
    /// Per player: its three motions (bound to its skeleton), the slot playing and the time in it.
    clips: Vec<[Option<Clip>; 3]>,
    slot: Vec<(usize, f32)>,
    gallery: Gallery,
    court_rng: Mt,
    sound_rng: Mt,
    voice: Option<Arc<SoundBank>>,
    flakes: Vec<Flake>,
    confetti: Option<Entity>,
    flake_rng: u32,
}

impl Ceremony {
    pub fn running(&self) -> bool {
        self.t < self.length
    }

    /// The camera at the clip's frame `t` (game space): eye, rows (right, down, forward), full FOV.
    fn view(&self) -> ([f32; 3], [[f32; 3]; 3], f32) {
        let (q, p, fov) = self.cam[(self.t as usize).min(self.cam.len() - 1)];
        let m = q_matrix(q);
        let turn = |r: [f32; 3]| if self.plan.turned { [-r[0], r[1], -r[2]] } else { r };
        let rows = [0, 1, 2].map(|k| turn([m[k][0], m[k][1], m[k][2]]));
        let p = turn([p[0], p[1], p[2]]);
        let o = self.plan.offset;
        ([p[0] + o[0], p[1] + o[1], p[2] + o[2]], rows, fov)
    }

    fn cosmetic(&mut self) -> f32 {
        // ponytail: a xorshift of its own for the confetti (the game's comes from its global rand)
        let mut x = self.flake_rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.flake_rng = x;
        x as f32 / u32::MAX as f32
    }
}

#[derive(Component)]
pub(super) struct Confetti;

/// Builds the ceremony when the match has ended: the plan, the clips, the voice bank, the confetti.
#[allow(clippy::too_many_arguments)]
pub(super) fn begin(
    mut commands: Commands,
    args: Res<crate::Args>,
    g: Res<super::super::Game>,
    screen: Res<super::Screen>,
    cer: Option<Res<Ceremony>>,
    art: Option<Res<super::Art>>,
    camera: Query<Entity, With<crate::Orbit>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(table) = g.match_stats.result.as_ref() else { return };
    if screen.open || cer.is_some() || std::env::var("HST_STATS_PAGE").is_ok_and(|p| p == "1") {
        return;
    }
    let Some((swapped, sound_rng)) = ENDING.lock().unwrap().take() else { return };
    let n = g.players.len();
    let humans: Vec<bool> = (0..n).map(|i| g.humans.get(i).copied().unwrap_or(false)).collect();
    let winner = table.winner.min(1);
    let plan = plan(&humans, &g.chars, winner, swapped, table.rows[winner][8]);
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let cam_name = ["win_99", "lost_99", "hand_99"][plan.clip];
    let data = iso.read("AZUMA/C_EFF/EFFCT.XB0").expect("EFFCT on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let e = arc.entries.iter().find(|e| e.name.replace('\\', "/").ends_with(&format!("prize/CAM/{cam_name}.CAM"))).expect("ceremony camera");
    let (cam, fov) = ani::parse_cam(&arc.read(e).expect("archive bytes")).expect("camera clip");
    let keys = |l: &Vec<(i32, [f32; 4])>| l.iter().map(|k| k.1).collect::<Vec<_>>();
    let (rot, pos, fov) = (keys(&cam.tracks[0].rotation), keys(&cam.tracks[0].position), keys(&fov.tracks[0].position));
    let cam: Vec<_> = (0..rot.len()).map(|k| (rot[k], pos[k], fov[k][0])).collect();
    let length = (cam.len() as u32).saturating_sub(1);
    let raw: Vec<Vec<u8>> = ["AZUMA/PRIZE/ANI/WINANI.XB", "AZUMA/PRIZE/ANI/LOSTANI.XB"].iter().filter_map(|p| iso.read(p).ok()).collect();
    let archives: Vec<Archive> = raw.iter().filter_map(|d| Archive::parse(d).ok()).collect();
    let motion = |name: &str| {
        let tail = format!("pcani/{}.ani2", name.to_ascii_lowercase());
        archives.iter().find_map(|a| {
            let e = a.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(&tail))?;
            ani::parse(&a.read(e).ok()?).ok()
        })
    };
    // ponytail: a mod character beyond the 14 plays stock character 13's files on its own skeleton
    let clips = (0..n).map(|i| std::array::from_fn(|s| motion(&plan.files[i][s]).map(|a| Clip::new(&g.data[i].skeleton, &a)))).collect();
    let slot = plan.start.iter().map(|&s| (s, 0.0)).collect();
    let c = g.chars[plan.shown].clamp(0, 13);
    let b = if g.stage == 7 { "B" } else { "" };
    let voice = SoundBank::load(&mut iso, &format!("SND/VOICE/PRIZE/PC_HV{c:02}{b}.XB0"), &format!("data/sound/VOICE/PRIZE/sgv_vc{c:02}f{}.hd", b.to_lowercase()))
        .map(Arc::new);
    let mut cer = Ceremony {
        plan,
        t: 0,
        length,
        cam,
        clips,
        slot,
        gallery: g.gallery.clone(),
        court_rng: g.rng.court.clone(),
        sound_rng,
        voice,
        flakes: Vec::new(),
        confetti: None,
        flake_rng: 0x2545_f491,
    };
    if let (false, Ok(cam), Some(art)) = (cer.plan.lost, camera.single(), art) {
        let root = commands.spawn((Transform::default(), Visibility::default(), Confetti)).id();
        commands.entity(cam).add_child(root);
        let mesh = meshes.add(Rectangle::new(FLAKE, FLAKE));
        for _ in 0..FLAKES {
            let r = |c: &mut Ceremony| c.cosmetic() * 2.0 - 1.0;
            let at = Vec3::new(r(&mut cer) * BOX.x, r(&mut cer) * BOX.y, -AHEAD + r(&mut cer) * BOX.z);
            let axis = Vec3::new(r(&mut cer), r(&mut cer), r(&mut cer)).try_normalize().unwrap_or(Vec3::Y);
            let hue = cer.cosmetic() * 360.0;
            let material = materials.add(StandardMaterial {
                base_color: Color::hsl(hue, 1.0, 0.6),
                base_color_texture: Some(art.0[super::CONFETTI].clone()),
                unlit: true,
                cull_mode: None,
                alpha_mode: AlphaMode::Blend,
                ..default()
            });
            let phase = r(&mut cer) * std::f32::consts::PI;
            let spin = cer.cosmetic() * std::f32::consts::TAU;
            let entity = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material), Transform::from_translation(at))).id();
            commands.entity(root).add_child(entity);
            cer.flakes.push(Flake { at, axis, spin, phase, entity });
        }
        cer.confetti = Some(root);
    }
    commands.insert_resource(cer);
}

/// The voice (`lost` program, key a bit of the sound generator) and the gallery's ovation or groan.
fn shout(c: &mut Ceremony, sound: Option<&Sound>) {
    let bit = c.sound_rng.next() >> 16 & 1;
    if let (Some(s), Some(v)) = (sound, &c.voice) {
        s.play_centre(v, sound::Play { slot: 1, program: c.plan.lost as u8, key: bit as u8, volume: 0x80, speed: 1.0 });
    }
    let roll = &mut || c.court_rng.next();
    c.gallery.point(Reaction { cheer: true, event: Some(if c.plan.lost { 4 } else { 3 }), chain: true }, roll);
}

/// One ceremony tick: its sounds, the motions, the confetti.
pub(super) fn tick(c: &mut Ceremony, players: u32, court: u8, sound: Option<&Sound>, gallery: Option<&Arc<SoundBank>>) {
    if c.t == 0 {
        if c.plan.voice_now {
            shout(c, sound);
        }
        if let (false, Some(s), Some(b)) = (c.plan.lost, sound, gallery) {
            for angle in CHEERS {
                s.play_toward(b, sound::Play { slot: 6, program: 10, key: 1, volume: CHEER_VOLUME, speed: 1.0 }, angle);
            }
        }
    }
    if let Some(s) = sound.filter(|_| !c.plan.lost) {
        s.music_volume((BGM * FADE.saturating_sub(c.t + 1) as f32 / FADE as f32) as u32);
    }
    c.t += 1;
    for i in 0..c.slot.len() {
        let (s, t) = c.slot[i];
        let len = c.clips[i][s].as_ref().map_or(0.0, |c| c.length);
        let t = t + 1.0;
        c.slot[i] = match s {
            _ if t < len => (s, t),
            WIN => (WIN, 10.0),
            HANDS => {
                if i == c.plan.shown {
                    shout(c, sound);
                }
                (WIN, 0.0)
            }
            _ => (s, len),
        };
    }
    let roll = &mut || c.court_rng.next();
    for (p, angle) in c.gallery.step(court, players, false, roll) {
        if let (Some(s), Some(b)) = (sound, gallery) {
            s.play_toward(b, p, angle);
        }
    }
    for f in &mut c.flakes {
        f.at.y -= FALL;
        f.phase += SWAY;
        if f.phase > std::f32::consts::PI {
            f.phase -= std::f32::consts::TAU;
        }
        f.spin += 20f32.to_radians();
        let centre = Vec3::new(0.0, 0.0, -AHEAD);
        let d = f.at - centre;
        f.at = centre + Vec3::new(wrap(d.x, BOX.x), wrap(d.y, BOX.y), wrap(d.z, BOX.z));
    }
}

fn wrap(v: f32, half: f32) -> f32 {
    (v + half).rem_euclid(2.0 * half) - half
}

/// The ceremony's view over the match camera (game space → Bevy: (x, −y, −z)).
pub(super) fn show(cer: Option<Res<Ceremony>>, mut camera: Query<(&mut Transform, &mut Projection), With<crate::Orbit>>, window: Query<&Window>) {
    let Some(c) = cer else { return };
    let Ok((mut t, mut proj)) = camera.single_mut() else { return };
    let (eye, rows, fov) = c.view();
    let b = |p: [f32; 3]| Vec3::new(p[0], -p[1], -p[2]);
    *t = Transform::from_translation(b(eye)).looking_to(b(rows[2]), -b(rows[1]));
    let aspect = window.single().map_or(4.0 / 3.0, |w| w.width() / w.height().max(1.0));
    if let Projection::Perspective(p) = &mut *proj {
        p.fov = 2.0 * ((fov / 2.0).tan() * super::super::SHOWN_ASPECT.max(1.0 / aspect)).atan();
    }
}

/// Every player on its mark in its ceremony motion (after the match's own pose), and the confetti.
pub(super) fn pose(
    cer: Option<Res<Ceremony>>,
    g: Res<super::super::Game>,
    mut figures: Query<(&super::super::Figure, &Rig, &mut Transform)>,
    mut joints: Query<&mut Transform, Without<super::super::Figure>>,
) {
    let Some(c) = cer else { return };
    for (f, rig, mut tf) in &mut figures {
        let i = f.0;
        let Some((at, far)) = c.plan.marks.get(i) else { continue };
        tf.translation = Vec3::from(*at);
        tf.rotation = if *far { Quat::from_rotation_y(std::f32::consts::PI) } else { Quat::IDENTITY };
        tf.scale = Vec3::new(g.players[i].hand, 1.0, 1.0);
        let (s, t) = c.slot[i];
        let Some(clip) = &c.clips[i][s] else { continue };
        for (j, joint) in rig.data.joints.iter().enumerate() {
            if let Ok(mut jt) = joints.get_mut(rig.joints[j]) {
                *jt = joint.rest;
            }
        }
        for (k, track) in clip.tracks.iter().enumerate() {
            let Ok(mut jt) = joints.get_mut(rig.joints[track.node]) else { continue };
            let (rot, pos) = clip.sample(k, t);
            if let Some([x, y, z, w]) = rot {
                jt.rotation = Quat::from_xyzw(-x, -y, -z, w).normalize();
            }
            if let Some(p) = pos {
                jt.translation = Vec3::from(p);
            }
        }
    }
    for f in &c.flakes {
        if let Ok(mut t) = joints.get_mut(f.entity) {
            let sway = Vec3::X * f.phase.sin() * 0.1;
            *t = Transform::from_translation(f.at + sway).with_rotation(Quat::from_axis_angle(f.axis, f.spin));
        }
    }
}

/// The ball, the score panel, the timing balloons and the markers stay off while the ceremony is up.
pub(super) fn hide(
    cer: Option<Res<Ceremony>>,
    mut q: Query<
        &mut Visibility,
        Or<(
            With<super::super::panel::Slot>,
            With<super::super::BalloonView>,
            With<super::super::markers::Marker>,
            With<super::super::BallView>,
        )>,
    >,
) {
    if cer.is_some() {
        for mut v in &mut q {
            *v = Visibility::Hidden;
        }
    }
}

/// Continue: the ceremony's confetti goes, the BGM comes back with the match's ticks.
pub(super) fn end(commands: &mut Commands, cer: Option<&Ceremony>) {
    if let Some(root) = cer.and_then(|c| c.confetti) {
        commands.entity(root).despawn();
    }
    commands.remove_resource::<Ceremony>();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The recorded doubles bot match (slot 5): team 0 won with a rating in the narrow band, the near end; the
    /// hand slap clip, team 0 shown at −6.4, players 0/2 slapping hands and 1/3 in their lost motions.
    #[test]
    fn bots_doubles() {
        let p = plan(&[false; 4], &[1, 4, 5, 9], 0, false, 60.0);
        assert_eq!((p.shown, p.lost, p.clip, p.turned, p.row, p.voice_now), (0, false, 2, false, 4, false));
        assert_eq!(p.offset, [0.0, 0.0, -6.4]);
        assert_eq!(p.start, vec![HANDS, LOST, HANDS, LOST]);
        assert_eq!(p.marks, vec![([0.75, 0.0, -6.4], false), ([-0.75, 0.0, 6.4], true), ([-0.75, 0.0, -6.4], false), ([0.75, 0.0, 6.4], true)]);
        assert_eq!(p.files[0], ["win/win_01", "hand/hand_00l_m", "lost/lost_01a"].map(String::from));
        assert_eq!(p.files[2][1..], ["hand/hand_01r", "lost/lost_05a"].map(String::from));
        // a 7 among the winners lowers the camera
        assert_eq!(plan(&[false; 4], &[7, 4, 5, 9], 0, false, 90.0).offset[1], -0.4);
    }

    /// Singles, a human beaten by the COM at the far end: the human's side shown, the lost clip lowered, turned
    /// to face it; the row the loser's.
    #[test]
    fn human_lost() {
        let p = plan(&[false, true], &[6, 11], 0, false, 70.0);
        assert_eq!((p.shown, p.lost, p.clip, p.turned, p.row, p.voice_now), (1, true, 1, true, 3, true));
        assert_eq!(p.offset, [0.0, -1.3, 6.4]);
        assert_eq!(p.files[1], ["win/win_11_n", "hand/hand_05l_n", "lost/lost_11b_n"].map(String::from));
        assert_eq!(p.start, vec![WIN, LOST]);
        // ends swapped, the human won: the win clip from the near end of team 1
        let p = plan(&[false, true], &[0, 0], 1, true, 85.0);
        assert_eq!((p.shown, p.clip, p.turned, p.row, p.offset[2]), (1, 0, false, 0, -6.4));
    }

    #[test]
    fn banner_and_plate() {
        let p = plan(&[false; 4], &[0; 4], 1, false, 70.0);
        let q = quads(&p, true);
        assert_eq!((q[0].src, q[0].dst), ([0.0, 160.0, 256.0, 80.0], [192.0, 360.0, 256.0, 80.0]));
        assert_eq!((q[1].rgb, q[7].src), ([67.0, 101.0, 127.0], [0.0, 32.0, 96.0, 32.0]));
        assert_eq!(quads(&p, false).len(), 1);
    }

    #[test]
    fn confetti_wraps() {
        assert_eq!(wrap(1.8, 1.7), -1.6000001);
        assert_eq!(wrap(-0.5, 1.7), -0.5);
    }
}
