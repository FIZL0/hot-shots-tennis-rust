//! The game's sound: SPU2 voices (`snd::Voice`) mixed into one 48 kHz stereo stream, playing bank sequences the
//! way the sound driver does — one sequence step a frame (800 samples), each key-on resolved to its tone, pitch and
//! L/R volume. `--sound <archive> <bank.hd> <program> <key>` auditions one sound. The music is a MIDI file on its
//! own bank, stepped a frame at a time by `snd::Sequencer`, each note-on resolved through the bank's programs.

use crate::Args;
use bevy::audio::{AddAudioSource, ChannelCount, Decodable, SampleRate, Source};
use bevy::prelude::*;
use hst_data::{exe, iso::Iso, snd::{self, Bank, KeyOn, Level, Voice}, xb::Archive};
use hst_sim::sound;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// The listener sits behind the +z baseline instead of the original's fixed −z one (B230): positional sounds turn
/// round with the match camera this machine shows, so left and right stay on screen's left and right.
static TURNED: AtomicBool = AtomicBool::new(false);

pub fn set_turned(on: bool) {
    TURNED.store(on, Ordering::Relaxed);
}

/// A bearing (whole degrees) as heard from the listener's end.
fn heard(angle: i32) -> i32 {
    if TURNED.load(Ordering::Relaxed) { (angle + 180) % 360 } else { angle }
}

/// Output samples per 60 Hz frame.
const FRAME: usize = 48_000 / 60;

/// A sound bank: `.hd` header and `.bd` samples, or a mod's voice takes (`wav`, by program: mono samples and their
/// rate; empty on disc banks).
pub struct SoundBank {
    pub hd: Vec<u8>,
    pub bd: Vec<u8>,
    pub wav: std::collections::HashMap<usize, Vec<(Arc<[i16]>, u32)>>,
}

impl SoundBank {
    /// `hd` names the header inside the disc archive `xb`; the samples sit beside it as `.bd`.
    pub fn load(iso: &mut Iso, xb: &str, hd: &str) -> Option<Self> {
        let data = iso.read(xb).ok()?;
        let arc = Archive::parse(&data).ok()?;
        let read = |name: &str| arc.read(arc.entries.iter().find(|e| e.name.replace('\\', "/").ends_with(name))?).ok();
        Some(Self { hd: read(hd)?, bd: read(&hd.replace(".hd", ".bd"))?, wav: Default::default() })
    }

    /// A BGM: bank `data/sound/BGM/<dir>/<name>.hd` and the MIDI file beside it in `SND/BGM/<NAME>.XB`.
    pub fn music(iso: &mut Iso, dir: &str, name: &str) -> Option<(Self, Vec<u8>)> {
        let xb = format!("SND/BGM/{}.XB", name.to_uppercase());
        Some((Self::load(iso, &xb, &format!("data/sound/BGM/{dir}/{name}.hd"))?, midi(iso, &xb)?))
    }

    /// A mod's voice (standard §5): `dir/<program>_<key>.wav`, keys in order; `None` if there is none.
    #[allow(dead_code)]
    pub fn wavs(dir: &std::path::Path) -> Option<Self> {
        let mut wav: std::collections::HashMap<usize, Vec<(usize, (Arc<[i16]>, u32))>> = Default::default();
        for e in crate::modfs::read_dir(dir) {
            let name = e.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
            let Some((p, k)) = name.strip_suffix(".wav").and_then(|s| s.split_once('_')) else { continue };
            let (Ok(p), Ok(k)) = (p.parse(), k.parse()) else { continue };
            match crate::modfs::read(&e).ok().and_then(|d| pcm(&d)) {
                Some((pcm, rate)) => wav.entry(p).or_default().push((k, (pcm.into(), rate))),
                None => warn!("{}: not a PCM16 wav, skipped", e.display()),
            }
        }
        let wav: std::collections::HashMap<_, _> = wav.into_iter().map(|(p, mut ks)| { ks.sort_by_key(|k| k.0); (p, ks.into_iter().map(|k| k.1).collect()) }).collect();
        (!wav.is_empty()).then(|| Self { hd: Vec::new(), bd: Vec::new(), wav })
    }
}

/// A RIFF PCM16 wav as mono samples (channels averaged) and its sample rate.
// ponytail: PCM16 only (what the mod tools write)
pub(crate) fn pcm(d: &[u8]) -> Option<(Vec<i16>, u32)> {
    if d.get(0..4)? != b"RIFF" || d.get(8..12)? != b"WAVE" {
        return None;
    }
    let (mut at, mut fmt, mut data) = (12, None, None);
    while let Some(h) = d.get(at..at + 8) {
        let len = u32::from_le_bytes(h[4..8].try_into().unwrap()) as usize;
        let body = d.get(at + 8..(at + 8 + len).min(d.len()))?;
        match &h[0..4] {
            b"fmt " => fmt = Some((u16::from_le_bytes(body.get(0..2)?.try_into().ok()?), u16::from_le_bytes(body.get(2..4)?.try_into().ok()?), u32::from_le_bytes(body.get(4..8)?.try_into().ok()?), u16::from_le_bytes(body.get(14..16)?.try_into().ok()?))),
            b"data" => data = Some(body),
            _ => {}
        }
        at += 8 + len + (len & 1);
    }
    let ((1, ch @ 1.., rate @ 1.., 16), Some(data)) = (fmt?, data) else { return None };
    let ch = ch as usize;
    let mono = data.chunks_exact(2 * ch).map(|f| (f.chunks_exact(2).map(|s| i16::from_le_bytes([s[0], s[1]]) as i32).sum::<i32>() / ch as i32) as i16);
    Some((mono.collect(), rate))
}

/// The tone every disc voice take keys on (all 14 characters' banks): set volume 127, tone volume 127, centred,
/// attack 0x80ff, release 0x9fe0. A mod's take plays through it at velocity 127.
const VOICE_TONE: snd::Tone = snd::Tone { raw: [0, 0, 0, 0, 0, 0, 0xff, 0x80, 0xe0, 0x9f, 1, 127, 64, 0, 0, 0] };

/// The SPU pitch register that plays `rate` Hz samples at play-speed word `scale` (0x1000 = as recorded), scaled
/// like a disc tone's (`snd::pitch`).
fn wav_pitch(rate: u32, scale: u32) -> u16 {
    let base = (rate as u64 * 4096 + 24_000) / 48_000;
    ((scale & 0xffff) as u64 * base >> 12) as u16
}

/// The MIDI file in archive `xb`.
fn midi(iso: &mut Iso, xb: &str) -> Option<Vec<u8>> {
    let data = iso.read(xb).ok()?;
    let arc = Archive::parse(&data).ok()?;
    arc.read(arc.entries.iter().find(|e| e.name.ends_with(".mid"))?).ok()
}

/// The BGM playing: its bank, track, and per MIDI channel the program, volume (CC7), pan (CC10) and expression (CC11).
struct Music {
    bank: Arc<SoundBank>,
    seq: snd::Sequencer,
    channels: [[u8; 4]; 16],
    /// The sequence volume (the game's BGM volume, 55 at full).
    volume: u32,
    /// Channel `c`'s voices carry play id `id + c`.
    id: u64,
}

struct Playing {
    id: u64,
    bank: Arc<SoundBank>,
    events: Vec<KeyOn>,
    next: usize,
    frame: u32,
    level: Level,
    /// Play-speed scale word (0x1000 = as recorded).
    scale: u32,
}

struct Mix {
    pitch: Vec<u16>,
    pan: Vec<u16>,
    gain: Vec<u16>,
    stereo: [Vec<i32>; 2],
    bank_volumes: Vec<u32>,
    // ponytail: every key-on gets its own voice; the driver's 48-voice allocation by priority is not ported
    /// Each voice with its bank, play id, note, pitch word (a wav take's: its rate) and level (for `Sound::update`).
    voices: Vec<(Voice, Arc<SoundBank>, u64, u8, u32, Level)>,
    playing: Vec<Playing>,
    music: Option<Music>,
    ids: u64,
    tags: Tags,
    /// B77: the plays the match re-levels every frame (`update`, `turn`: the flight whistle and rush, the flyover).
    followed: Vec<u64>,
    vol: Vol,
    /// B203: the menu cues sounding, cut when the next of their lane starts; B307: [cursor move, every other cue].
    menu: [u64; 2],
}

/// B235: the volume settings' groups. Presentation only: applied to the output, never to a play's level.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    /// The BGM and the jingles.
    Music,
    /// Court sounds, the crowd, weather, menu and system sounds.
    Sfx,
    /// The players' and the umpire's voices.
    Voices,
}

impl Group {
    /// The group of a play on bank slot `slot`: 1..=4 the players' voices, 5 the umpire's, 0x16 the select's voice
    /// previews, 8 the jingles; the rest (court 0, gallery 6, system 9) sound effects.
    /// ponytail: character 9's lob shout is on the system bank (slot 9 key 7, `hst_sim::sound`) and counts as an SFX
    pub fn of(slot: u8) -> Self {
        match slot {
            1..=5 | 0x16 => Group::Voices,
            8 => Group::Music,
            _ => Group::Sfx,
        }
    }
}

/// B235: the four volume sliders in tenths (0..=10): master, music, sound effects, voices. Saved in settings.txt.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Volumes(pub [u8; 4]);

impl Default for Volumes {
    fn default() -> Self {
        Volumes([10; 4])
    }
}

impl Volumes {
    pub const NAMES: [&str; 4] = ["master_volume", "music_volume", "sfx_volume", "voice_volume"];

    /// The volume lines of a settings.txt (others ignored; missing ones full).
    pub fn parse(text: &str) -> Self {
        let mut v = Volumes::default();
        for (k, x) in text.lines().filter_map(|l| l.split('#').next()?.split_once('=')) {
            if let (Some(i), Ok(x)) = (Self::NAMES.iter().position(|n| *n == k.trim()), x.trim().parse::<u8>()) {
                v.0[i] = x.min(10);
            }
        }
        v
    }

    pub fn text(&self) -> String {
        Self::NAMES.iter().zip(self.0).map(|(n, x)| format!("{n} = {x}\n")).collect()
    }

    /// Each group's gain (music, SFX, voices), the master's included.
    fn gains(&self) -> [f32; 3] {
        let [m, g @ ..] = self.0.map(|x| x as f32 / 10.0);
        g.map(|g| m * g)
    }
}

/// Each play's group and the groups' gains.
struct Vol {
    groups: std::collections::HashMap<u64, Group>,
    gain: [f32; 3],
}

impl Default for Vol {
    fn default() -> Self {
        Vol { groups: Default::default(), gain: [1.0; 3] }
    }
}

/// The match's sounds (B111): every play started while a match runs and isn't paused (SFX, voices, crowd, weather;
/// not the BGM, which the mode switch stops, nor the pause screen's own sounds). `held` freezes them in place.
#[derive(Default)]
struct Tags {
    on: bool,
    held: bool,
    ids: std::collections::HashSet<u64>,
}

impl Mix {
    fn follow(&mut self, id: u64) {
        if !self.followed.contains(&id) {
            self.followed.push(id);
        }
    }

    /// The play call's part of the level for `p` at `pos`.
    fn level(&self, p: sound::Play, pos: [f32; 3]) -> Level {
        // turned: the listener's end and facing turned round the court's centre, as the sound's spot the other way
        let pos = if TURNED.load(Ordering::Relaxed) { [-pos[0], pos[1], -pos[2]] } else { pos };
        let (angle, dist) = sound::place(pos);
        let seq = sound::stereo(sound::falloff(p.volume, dist), angle, &self.stereo).map(|x| x as u32);
        Level { seq, bank: self.bank_volumes[p.slot as usize], pan: [0x40; 3], ..default() }
    }

    /// One frame: the sequences' events due now, then `FRAME` stereo samples.
    fn render(&mut self, out: &mut Vec<f32>) {
        let mut playing = std::mem::take(&mut self.playing);
        for p in &mut playing {
            if self.tags.held && self.tags.ids.contains(&p.id) {
                continue;
            }
            while let Some(&e) = p.events.get(p.next).filter(|e| e.frame == p.frame) {
                self.key_on(p, e);
                p.next += 1;
            }
            p.frame += 1;
        }
        playing.retain(|p| p.next < p.events.len());
        self.playing = playing;
        if let Some(mut m) = self.music.take() {
            for e in m.seq.frame() {
                self.midi(&mut m, e);
            }
            self.music = Some(m);
        }
        // B235: each voice summed into its group, the groups scaled by their volume settings (all full: as before)
        let groups: Vec<usize> = self.voices.iter().map(|v| self.vol.groups.get(&v.2).map_or(Group::Sfx, |g| *g) as usize).collect();
        let gain = self.vol.gain;
        for _ in 0..FRAME {
            let mut s = [[0; 2]; 3];
            let tags = &self.tags;
            for ((v, b, ..), &g) in self.voices.iter_mut().zip(&groups).filter(|(v, _)| !(tags.held && tags.ids.contains(&v.2))) {
                let [l, r] = v.tick(&b.bd);
                s[g] = [s[g][0] + l, s[g][1] + r];
            }
            // ponytail: dry voices only — no core/master volume or reverb
            out.extend([0, 1].map(|c| (0..3).map(|g| s[g][c] as f32 * gain[g]).sum::<f32>().clamp(-32768.0, 32767.0) / 32768.0));
        }
        self.voices.retain(|v| v.0.phase != 0);
        let (voices, playing) = (&self.voices, &self.playing);
        self.tags.ids.retain(|&id| playing.iter().any(|q| q.id == id) || voices.iter().any(|v| v.2 == id));
        let music = self.music.as_ref().map(|m| m.id..m.id + 16);
        self.vol.groups.retain(|id, _| music.as_ref().is_some_and(|r| r.contains(id)) || playing.iter().any(|q| q.id == *id) || voices.iter().any(|v| v.2 == *id));
    }

    /// A new play id, tagged as the match's while one runs unpaused.
    fn id(&mut self) -> u64 {
        self.ids += 1;
        if self.tags.on && !self.tags.held {
            self.tags.ids.insert(self.ids);
        }
        self.ids
    }

    /// One MIDI message of the BGM, as the sound driver takes it.
    // ponytail: controller changes apply to later notes only; the driver also re-levels the channel's sounding voices
    fn midi(&mut self, m: &mut Music, [status, a, b]: [u8; 3]) {
        let c = (status & 0xf) as usize;
        let id = m.id + c as u64;
        let ch = &mut m.channels[c];
        match status & 0xf0 {
            0x90 if b > 0 => {
                let Ok(bank) = Bank::parse(&m.bank.hd) else { return };
                let (tones, program_volume) = bank.program(ch[0] as usize, a);
                if ch[1] == 0 || ch[3] == 0 {
                    return;
                }
                for t in tones {
                    let mut level = Level { seq: [m.volume; 2], bank: 127, velocity: b as u32, pan: [0x40, ch[2].clamp(1, 127) as u32, 0], ..default() };
                    level.program(ch[1], ch[3], program_volume, t, &self.gain);
                    let word = (t.root() as u32) << 24 | (a as u32) << 16 | (t.fine() as u8 as u32) << 8 | 0x40;
                    let pitch = snd::pitch(&self.pitch, word, 0x0100_1000);
                    let voice = Voice::key_on(t.sample() as u32 / 2, t.adsr(), pitch, level.volume(&self.pan));
                    self.voices.push((voice, m.bank.clone(), id, a, word, level));
                }
            }
            0x80 | 0x90 => self.voices.iter_mut().filter(|v| v.2 == id && v.3 == a).for_each(|v| v.0.key_off()),
            0xb0 => match a {
                7 => ch[1] = b,
                10 => ch[2] = b,
                11 => ch[3] = b,
                _ => {} // ponytail: bank select and data entry (the reverb's parameters) do nothing here
            },
            0xc0 => ch[0] = a,
            _ => {}
        }
    }

    fn key_on(&mut self, p: &Playing, e: KeyOn) {
        if e.velocity == 0 {
            self.voices.iter_mut().filter(|v| v.2 == p.id && v.3 == e.note).for_each(|v| v.0.key_off());
            return;
        }
        let Ok(b) = Bank::parse(&p.bank.hd) else { return };
        let (Some(t), Some(set)) = (b.tone(e.set as usize, e.note), b.set_volume(e.set as usize)) else { return };
        let mut level = p.level;
        level.velocity = e.velocity as u32; // ponytail: raw velocity, the driver's velocity curve is not ported
        level.tone(set, t, &self.gain);
        let word = (t.root() as u32) << 24 | (e.note as u32) << 16 | (t.fine() as u8 as u32) << 8 | 0x40;
        let pitch = snd::pitch(&self.pitch, word, 0x0100_0000 | p.scale);
        let voice = Voice::key_on(t.sample() as u32 / 2, t.adsr(), pitch, level.volume(&self.pan));
        self.voices.push((voice, p.bank.clone(), p.id, e.note, word, level));
    }
}

/// The mixer the game plays its sounds on.
#[derive(Resource, Clone)]
pub struct Sound(Arc<Mutex<Mix>>);

impl Sound {
    /// Plays `(program, key)` of `bank`. `level` holds the play call's part: sequence volume per side, bank volume
    /// and the play and channel pans (`pan[0..2]`); the tone and velocity parts come from the sequence.
    /// Returns the play's id for `update` and `stop` (0 if the bank has no such sequence).
    pub fn play(&self, bank: &Arc<SoundBank>, program: usize, key: usize, level: Level, scale: u32) -> u64 {
        self.play_as(bank, program, key, level, scale, Group::Sfx)
    }

    /// `play` in volume group `group`.
    fn play_as(&self, bank: &Arc<SoundBank>, program: usize, key: usize, level: Level, scale: u32, group: Group) -> u64 {
        // a mod's take: a voice keyed on at once with the disc voices' tone at velocity 127, at the wav's rate; a
        // program the mod ships no wav for falls to `hd`, empty on a mod bank (B181: never its donor's), so silence
        // ponytail: a key past the takes wraps round them (the game only asks for keys its disc bank has)
        if let Some((w, rate)) = bank.wav.get(&program).and_then(|ks| ks.get(key % ks.len())) {
            let mut m = self.0.lock().unwrap();
            let id = m.id();
            m.vol.groups.insert(id, group);
            let mut level = Level { velocity: 127, ..level };
            level.tone(127, VOICE_TONE, &m.gain);
            let voice = Voice::key_on_pcm(w.clone(), VOICE_TONE.adsr(), wav_pitch(*rate, scale), level.volume(&m.pan));
            m.voices.push((voice, bank.clone(), id, 0, *rate, level));
            return id;
        }
        let Some(events) = Bank::parse(&bank.hd).ok().and_then(|b| b.key_ons(program, key)) else { return 0 };
        let mut m = self.0.lock().unwrap();
        let id = m.id();
        m.vol.groups.insert(id, group);
        m.playing.push(Playing { id, bank: bank.clone(), events, next: 0, frame: 0, level, scale });
        id
    }

    /// Starts a BGM from the top at `volume` (the game's 55), replacing the one playing.
    pub fn music(&self, bank: &Arc<SoundBank>, mid: &[u8], volume: u32) {
        let mut m = self.0.lock().unwrap();
        if let Some(old) = m.music.take() {
            m.voices.iter_mut().filter(|v| (old.id..old.id + 16).contains(&v.2)).for_each(|v| v.0.key_off());
        }
        let Some(seq) = snd::Sequencer::new(mid) else { return };
        let id = m.ids + 1;
        m.ids += 16;
        m.vol.groups.extend((id..id + 16).map(|c| (c, Group::Music)));
        m.music = Some(Music { bank: bank.clone(), seq, channels: [[0, 100, 0x40, 127]; 16], volume, id });
    }

    /// B235: the volume settings, applied at once to everything sounding and to come.
    pub fn set_volumes(&self, v: Volumes) {
        self.0.lock().unwrap().vol.gain = v.gains();
    }

    /// The BGM playing is one of `bank`'s (the court intro starts the match's BGM before the match ticks do).
    pub fn music_of(&self, bank: &Arc<SoundBank>) -> bool {
        self.0.lock().unwrap().music.as_ref().is_some_and(|m| Arc::ptr_eq(&m.bank, bank))
    }

    /// Stops the BGM: its sequence and its sounding notes.
    pub fn stop_music(&self) {
        let mut m = self.0.lock().unwrap();
        if let Some(old) = m.music.take() {
            m.voices.retain(|v| !(old.id..old.id + 16).contains(&v.2));
        }
    }

    /// The BGM's sequence volume, its sounding notes included.
    pub fn music_volume(&self, volume: u32) {
        let mut m = self.0.lock().unwrap();
        let m = &mut *m;
        let Some(music) = &mut m.music else { return };
        music.volume = volume;
        for (v, .., level) in m.voices.iter_mut().filter(|v| (music.id..music.id + 16).contains(&v.2)) {
            level.seq = [volume; 2];
            v.volume = level.volume(&m.pan);
        }
    }

    /// The play `id` moved to `pos` at play speed `speed`: its voices re-placed (volume `p.volume`) and re-pitched.
    pub fn update(&self, id: u64, p: sound::Play, pos: [f32; 3]) {
        let mut m = self.0.lock().unwrap();
        let seq = m.level(p, pos).seq;
        let scale = sound::speed_word(p.speed);
        let m = &mut *m;
        m.follow(id);
        for q in m.playing.iter_mut().filter(|q| q.id == id) {
            (q.level.seq, q.scale) = (seq, scale);
        }
        for (v, .., word, level) in m.voices.iter_mut().filter(|v| v.2 == id) {
            level.seq = seq;
            v.volume = level.volume(&m.pan);
            v.pitch = if v.pcm.is_some() { wav_pitch(*word, scale) } else { snd::pitch(&m.pitch, *word, 0x0100_0000 | scale) };
        }
    }

    /// A match starts: from here its plays are tagged as match sounds.
    pub fn begin_match(&self) {
        self.0.lock().unwrap().tags.on = true;
    }

    /// The pause screen opened (`true`) or closed: the match's sounds freeze where they are, or go on. Sounds
    /// started while held (the pause screen's) aren't the match's.
    pub fn hold(&self, held: bool) {
        self.0.lock().unwrap().tags.held = held;
    }

    /// The match is left (any exit: quit, match end, disconnect): all its sounds cut, nothing tagged from here.
    pub fn end_match(&self) {
        let mut m = self.0.lock().unwrap();
        let ids = std::mem::take(&mut m.tags).ids;
        m.playing.retain(|q| !ids.contains(&q.id));
        m.voices.retain(|v| !ids.contains(&v.2));
    }

    /// Match sounds still playing or held.
    pub fn match_sounds(&self) -> usize {
        self.0.lock().unwrap().tags.ids.len()
    }

    /// A mixer with no tables and no output, for tests that only follow plays.
    #[cfg(test)]
    pub fn bare() -> Self {
        Self(Arc::new(Mutex::new(Mix { pitch: Vec::new(), pan: Vec::new(), gain: Vec::new(), stereo: Default::default(), bank_volumes: Vec::new(), voices: Vec::new(), playing: Vec::new(), music: None, ids: 0, tags: Tags::default(), followed: Vec::new(), vol: Vol::default(), menu: [0; 2] })))
    }

    /// A play that never keys on and never ends (a weather loop's stand-in), for tests.
    #[cfg(test)]
    pub fn endless(&self) -> u64 {
        let mut m = self.0.lock().unwrap();
        let id = m.id();
        let bank = Arc::new(SoundBank { hd: Vec::new(), bd: Vec::new(), wav: Default::default() });
        m.playing.push(Playing { id, bank, events: vec![KeyOn { frame: u32::MAX, channel: 0, note: 0, velocity: 0, set: 0 }], next: 0, frame: 0, level: default(), scale: 0x1000 });
        id
    }

    /// A non-positional play (the umpire's voice): centred, sequence volume `p.volume`.
    /// ponytail: the library's non-positional level is taken as the positional one straight ahead with no falloff
    pub fn play_centre(&self, bank: &Arc<SoundBank>, p: sound::Play) -> u64 {
        let bank_volume = self.0.lock().unwrap().bank_volumes[p.slot as usize];
        let level = Level { seq: [p.volume as u32; 2], bank: bank_volume, pan: [0x40; 3], ..default() };
        self.play_as(bank, p.program as usize, p.key as usize, level, sound::speed_word(p.speed), Group::of(p.slot))
    }

    /// B203: a menu cue (system bank key `key`, centred, full volume) that cuts the one before it, so mashing
    /// buttons or a held direction's repeat never stacks cues louder than one press. B307: the cursor move (key 0)
    /// only cuts the cursor move before it, so a select/back cue still sounding plays out under it; any other cue
    /// cuts both.
    pub fn play_menu(&self, bank: &Arc<SoundBank>, key: u8) -> u64 {
        let lane = usize::from(key != 0);
        let mut m = self.0.lock().unwrap();
        let menu = m.menu;
        let cut = &menu[..lane + 1];
        m.playing.retain(|q| !cut.contains(&q.id));
        m.voices.retain(|v| !cut.contains(&v.2));
        drop(m);
        let id = self.play_centre(bank, sound::Play { slot: 9, program: 0, key, volume: 0x80, speed: 1.0 });
        self.0.lock().unwrap().menu[lane] = id;
        id
    }

    /// How many 60 Hz frames `play_centre(bank, p)` sounds for: rendered alone on a silent copy of the mixer, so
    /// the match can time the umpire's voice by ticks, the same on every machine (the live mixer runs on the audio
    /// device's clock). ponytail: capped at 30 s; a looping sequence would never end
    pub fn centre_frames(&self, bank: &Arc<SoundBank>, p: sound::Play) -> u32 {
        let solo = {
            let m = self.0.lock().unwrap();
            Mix { pitch: m.pitch.clone(), pan: m.pan.clone(), gain: m.gain.clone(), stereo: m.stereo.clone(), bank_volumes: m.bank_volumes.clone(), voices: Vec::new(), playing: Vec::new(), music: None, ids: 0, tags: Tags::default(), followed: Vec::new(), vol: Vol::default(), menu: [0; 2] }
        };
        let solo = Self(Arc::new(Mutex::new(solo)));
        let id = solo.play_centre(bank, p);
        let mut out = Vec::new();
        let mut n = 0;
        while id != 0 && solo.playing(id) && n < 30 * 60 {
            out.clear();
            solo.0.lock().unwrap().render(&mut out);
            n += 1;
        }
        n
    }

    /// A mixer on `iso`'s banks with no output, for tests.
    #[cfg(test)]
    pub fn test(iso: &mut Iso) -> Self {
        Self(Arc::new(Mutex::new(mix(iso))))
    }

    /// How many plays have started (the BGM counts 16), for tests.
    #[cfg(test)]
    pub fn plays(&self) -> u64 {
        self.0.lock().unwrap().ids
    }

    /// B77: the match paused. As the original's per-frame update does while paused, every play the match re-levels
    /// each frame is held at play speed 0 (pitch 0: it stops where it is, silent); the match's next `update` after
    /// it resumes picks it up again. Every other voice (one-shots, the crowd, the BGM) plays on, as there.
    pub fn hold_followed(&self) {
        let mut m = self.0.lock().unwrap();
        let Mix { voices, playing, followed, .. } = &mut *m;
        followed.retain(|id| voices.iter().any(|v| v.2 == *id) || playing.iter().any(|q| q.id == *id));
        playing.iter_mut().filter(|q| followed.contains(&q.id)).for_each(|q| q.scale = 0);
        voices.iter_mut().filter(|v| followed.contains(&v.2)).for_each(|v| v.0.pitch = 0);
    }

    /// The play `id` still has key-ons to come or voices sounding.
    pub fn playing(&self, id: u64) -> bool {
        let m = self.0.lock().unwrap();
        m.playing.iter().any(|q| q.id == id) || m.voices.iter().any(|v| v.2 == id)
    }

    /// Ends the play `id`: no more key-ons, its voices released.
    pub fn stop(&self, id: u64) {
        let mut m = self.0.lock().unwrap();
        m.playing.retain(|q| q.id != id);
        m.voices.iter_mut().filter(|v| v.2 == id).for_each(|v| v.0.key_off());
    }

    /// A game sound at `pos` (game space): bearing and falloff from the fixed listener, L/R from the stereo
    /// tables, the bank volume of the play's slot and its play speed.
    pub fn play_at(&self, bank: &Arc<SoundBank>, p: sound::Play, pos: [f32; 3]) -> u64 {
        let level = self.0.lock().unwrap().level(p, pos);
        self.play_as(bank, p.program as usize, p.key as usize, level, sound::speed_word(p.speed), Group::of(p.slot))
    }

    /// The play `id` (a `play_toward` one) turned to `angle`: its voices re-panned at sequence volume `volume`.
    pub fn turn(&self, id: u64, volume: i32, angle: i32) {
        let mut m = self.0.lock().unwrap();
        let m = &mut *m;
        let seq = sound::stereo(volume, heard(angle), &m.stereo).map(|x| x as u32);
        m.follow(id);
        m.playing.iter_mut().filter(|q| q.id == id).for_each(|q| q.level.seq = seq);
        for (v, .., level) in m.voices.iter_mut().filter(|v| v.2 == id) {
            level.seq = seq;
            v.volume = level.volume(&m.pan);
        }
    }

    /// A sound from the fixed `angle` (whole degrees) at its full volume, as the gallery plays its stands.
    pub fn play_toward(&self, bank: &Arc<SoundBank>, p: sound::Play, angle: i32) -> u64 {
        let level = {
            let m = self.0.lock().unwrap();
            let seq = sound::stereo(p.volume, heard(angle), &m.stereo).map(|x| x as u32);
            Level { seq, bank: m.bank_volumes[p.slot as usize], pan: [0x40; 3], ..default() }
        };
        self.play_as(bank, p.program as usize, p.key as usize, level, sound::speed_word(p.speed), Group::of(p.slot))
    }
}

#[derive(Asset, TypePath)]
struct Stream(Arc<Mutex<Mix>>);

struct StreamDecoder {
    mix: Arc<Mutex<Mix>>,
    buf: Vec<f32>,
    at: usize,
}

impl Iterator for StreamDecoder {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.at == self.buf.len() {
            self.buf.clear();
            self.mix.lock().unwrap().render(&mut self.buf);
            self.at = 0;
        }
        self.at += 1;
        Some(self.buf[self.at - 1])
    }
}

impl Source for StreamDecoder {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        ChannelCount::new(2).unwrap()
    }
    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(48_000).unwrap()
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl Decodable for Stream {
    type Decoder = StreamDecoder;
    fn decoder(&self) -> StreamDecoder {
        StreamDecoder { mix: self.0.clone(), buf: Vec::new(), at: 0 }
    }
}

pub fn plugin(app: &mut App) {
    app.add_audio_source::<Stream>().add_systems(Startup, start);
}

fn start(mut commands: Commands, args: Res<Args>, mut streams: ResMut<Assets<Stream>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let sound = Sound(Arc::new(Mutex::new(mix(&mut iso))));
    // B235: the saved volumes (the menu's Sound screen sets them again as they change)
    let settings = crate::play::main_menu::Settings::path(&args.iso);
    sound.set_volumes(crate::store::read_to_string(settings).map_or_else(|_| Volumes::default(), |t| Volumes::parse(&t)));
    commands.spawn(AudioPlayer(streams.add(Stream(sound.0.clone()))));
    if let Some((xb, hd, program, key)) = &args.sound {
        let bank = Arc::new(SoundBank::load(&mut iso, xb, hd).expect("sound bank on disc"));
        if let Some(mid) = midi(&mut iso, xb) {
            sound.music(&bank, &mid, 55);
        }
        sound.play(&bank, *program, *key, Level { seq: [127; 2], bank: 127, pan: [0x40; 3], ..default() }, 0x1000);
    }
    // the court's sound effects (bank slot 0); court 0 has no bank of its own
    let n = args.stage.map_or(args.court, |s| s as usize);
    commands.insert_resource(court_bank(&mut iso, n));
    commands.insert_resource(sound);
}

/// Court `n`'s sound effects (bank slot 0); court 0 has none.
pub fn court_bank(iso: &mut Iso, n: usize) -> CourtBank {
    CourtBank(SoundBank::load(iso, &format!("SND/COURT/C_SND{n:02}A.XB0"), &format!("data/sound/SE/court/co_se{n:02}.hd")).map(Arc::new))
}

/// Court `n`'s gallery bank (slot 6) for the setup's pick 0..3 (`Rngs::setup_gallery`): archive A (`galsg`) below
/// 2, else B (`galdv`); crowd a or b by its low bit.
pub fn gallery_bank(iso: &mut Iso, n: usize, pick: usize) -> Option<SoundBank> {
    let (xb, set) = if pick < 2 { ('A', "sg") } else { ('B', "dv") };
    let crowd = if pick & 1 == 0 { 'a' } else { 'b' };
    SoundBank::load(iso, &format!("SND/COURT/C_SND{n:02}{xb}.XB0"), &format!("data/sound/VOICE/GALLERY/gal{set}{n:02}{crowd}.hd"))
}

/// The players' voice banks (slots 1 and up), from `play.rs`'s line-up.
#[derive(Resource)]
pub struct VoiceBanks(pub Vec<Option<Arc<SoundBank>>>);

/// Character `c`'s voice bank for a game of `players` on `court`: the singles (`sgv`) or doubles (`dvv`) set,
/// variant a or b (`b`); court 7 has its own pair, c and d.
pub fn voice_bank(iso: &mut Iso, c: usize, players: usize, court: u32, b: bool) -> Option<SoundBank> {
    let (n, set, tag) = if players < 3 { (0, "SGL", "sgv") } else { (4, "DBL", "dvv") };
    let k = b as usize + if court == 7 { 2 } else { 0 };
    let v = (b'a' + k as u8) as char;
    SoundBank::load(iso, &format!("SND/VOICE/PC/PC{c:02}VCE{}.XB0", n + k), &format!("data/sound/VOICE/{set}/{tag}_vc{c:02}{v}.hd"))
}

/// The gallery's bank.
#[derive(Resource)]
pub struct GalleryBank(pub Option<Arc<SoundBank>>);

/// The umpire's voice bank (slot 5): umpire `n`'s set `v` (0..3 = a..d).
pub fn umpire_bank(iso: &mut Iso, n: u8, v: u8) -> Option<SoundBank> {
    let (big, small) = ((b'A' + v) as char, (b'a' + v) as char);
    SoundBank::load(iso, &format!("SND/UMP/UV{n:02}{big}.XB0"), &format!("data/sound/UMPIRE/gag_vc{n:02}{small}.hd"))
}

/// The jingle bank (slot 8) and the BGM's bank and MIDI file.
#[derive(Resource)]
pub struct MusicBanks {
    pub jingles: Option<Arc<SoundBank>>,
    pub bgm: Option<(Arc<SoundBank>, Vec<u8>)>,
}

/// The court's sound-effect bank.
#[derive(Resource)]
pub struct CourtBank(pub Option<Arc<SoundBank>>);

fn mix(iso: &mut Iso) -> Mix {
    let pitch = exe::pitch_table(&iso.read("MODULES2/SG2IOPM1.IRX").expect("sound driver")).expect("supported disc");
    let elf = iso.read("SCUS_976.10").expect("main program");
    let (pan, gain) = exe::sound_tables(&elf).expect("supported disc");
    let stereo = exe::stereo_tables(&elf).expect("supported disc");
    let bank_volumes = exe::bank_volumes(&elf).expect("supported disc");
    Mix { pitch, pan, gain, stereo, bank_volumes, voices: Vec::new(), playing: Vec::new(), music: None, ids: 0, tags: Tags::default(), followed: Vec::new(), vol: Vol::default(), menu: [0; 2] }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every character's voice banks (0..13, singles and doubles, variants a and b, and court 7's c and d) load from
    /// the disc.
    #[test]
    fn loads_voice_banks() {
        let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let missing: Vec<_> = (0..14)
            .flat_map(|c| [2, 4].map(|players| [1, 7].map(|court| [false, true].map(|b| (c, players, court, b)))))
            .flatten()
            .flatten()
            .filter(|&(c, players, court, b)| voice_bank(&mut iso, c, players, court, b).is_none())
            .collect();
        assert!(missing.is_empty(), "{missing:?}");
    }

    /// The court BGM and the jingles load and play: the music sounds, a jingle sounds and ends.
    #[test]
    fn plays_music_and_jingles() {
        let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let sound = Sound(Arc::new(Mutex::new(mix(&mut iso))));
        for n in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 14] {
            assert!(SoundBank::music(&mut iso, "Court", &format!("bgmg_{n:02}")).is_some(), "bgmg_{n:02}");
        }
        let (bank, mid) = SoundBank::music(&mut iso, "Court", "bgmg_01").unwrap();
        let bank = Arc::new(bank);
        sound.music(&bank, &mid, 55);
        // the court intro's BGM is the match's: the match doesn't start it again
        assert!(sound.music_of(&bank) && !sound.music_of(&Arc::new(SoundBank::music(&mut iso, "Court", "bgmg_02").unwrap().0)));
        let peak = |out: &[f32]| out.iter().fold(0f32, |a, x| a.max(x.abs()));
        let mut out = Vec::new();
        for _ in 0..600 {
            sound.0.lock().unwrap().render(&mut out);
        }
        eprintln!("music peak {}, voices {}", peak(&out), sound.0.lock().unwrap().voices.len());
        assert!(peak(&out) > 0.01);
        sound.music_volume(0);
        let jingles = Arc::new(SoundBank::load(&mut iso, "SND/JGL/JIG_00.XB", "data/sound/JINGLE/jig_00.hd").unwrap());
        let id = sound.play_centre(&jingles, sound::Play { slot: 8, program: 0, key: 1, volume: 0x80, speed: 1.0 });
        let mut frames = 0;
        while sound.playing(id) && frames < 3600 {
            sound.0.lock().unwrap().render(&mut out);
            frames += 1;
        }
        eprintln!("jingle 1: {frames} frames");
        assert!(frames > 30 && frames < 3600);
    }

    /// B386: the match-end sounds of a win, a loss and an annihilation (the banner differs, the sounds don't): the
    /// result music `bgmm_10`, the win and loss jingles (keys 3, 4), every character's prize voice (program 0 won,
    /// 1 lost, either line, the B court's set too) and each court's gallery ovation and groan (programs 3/7, 4/8)
    /// load and are heard.
    #[test]
    fn match_end_sounds_are_heard() {
        let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let base = mix(&mut iso);
        // the loudest sample of one play over 2 s
        let peak = |bank: &Arc<SoundBank>, p: sound::Play| {
            let sound = Sound(Arc::new(Mutex::new(mix_clone(&base))));
            sound.play_centre(bank, p);
            let mut out = Vec::new();
            for _ in 0..120 {
                sound.0.lock().unwrap().render(&mut out);
            }
            out.iter().fold(0f32, |a, x| a.max(x.abs()))
        };
        let (bgm, mid) = SoundBank::music(&mut iso, "Menu", "bgmm_10").expect("result music");
        let sound = Sound(Arc::new(Mutex::new(mix_clone(&base))));
        sound.music(&Arc::new(bgm), &mid, 55);
        let mut out = Vec::new();
        for _ in 0..600 {
            sound.0.lock().unwrap().render(&mut out);
        }
        let music = out.iter().fold(0f32, |a, x| a.max(x.abs()));
        assert!(music > 0.01, "result music peak {music}");
        let jingles = Arc::new(SoundBank::load(&mut iso, "SND/JGL/JIG_00.XB", "data/sound/JINGLE/jig_00.hd").unwrap());
        for key in [3, 4] {
            let p = peak(&jingles, sound::Play { slot: 8, program: 0, key, volume: 0x80, speed: 1.0 });
            assert!(p > 0.01, "jingle {key} peak {p}");
        }
        for c in 0..14 {
            for b in ["", "B"] {
                let voice = SoundBank::load(&mut iso, &format!("SND/VOICE/PRIZE/PC_HV{c:02}{b}.XB0"), &format!("data/sound/VOICE/PRIZE/sgv_vc{c:02}f{}.hd", b.to_lowercase()));
                let voice = Arc::new(voice.unwrap_or_else(|| panic!("prize voice {c:02}{b}")));
                for (program, key) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                    let p = peak(&voice, sound::Play { slot: 1, program, key, volume: 0x80, speed: 1.0 });
                    assert!(p > 0.01, "character {c}{b} program {program} key {key} peak {p}");
                }
            }
        }
        for n in 1..=11 {
            for pick in 0..4 {
                let Some(gallery) = gallery_bank(&mut iso, n, pick) else { continue };
                let gallery = Arc::new(gallery);
                for (program, key) in [(3, 0), (7, 0), (7, 1), (4, 0), (8, 0)] {
                    let p = peak(&gallery, sound::Play { slot: 6, program, key, volume: 0x80, speed: 1.0 });
                    assert!(p > 0.01, "court {n} gallery {pick} program {program} key {key} peak {p}");
                }
            }
        }
    }

    /// B235: each volume slider at 0 silences only its group — the BGM (music), a system sound (SFX), the umpire
    /// (voices) — and master silences all; at full the output is as before (no gain applied).
    #[test]
    fn each_volume_silences_its_group() {
        let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let base = mix(&mut iso);
        let (bgm, mid) = SoundBank::music(&mut iso, "Court", "bgmg_01").unwrap();
        let (bgm, sys) = (Arc::new(bgm), Arc::new(SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd").unwrap()));
        let umpire = Arc::new(umpire_bank(&mut iso, 0, 0).unwrap());
        // the loudest sample of `group`'s source alone over 2 s at volumes `v`
        let peak = |group: Group, v: [u8; 4]| {
            let sound = Sound(Arc::new(Mutex::new(mix_clone(&base))));
            sound.set_volumes(Volumes(v));
            match group {
                Group::Music => sound.music(&bgm, &mid, 55),
                Group::Sfx => _ = sound.play_centre(&sys, sound::Play { slot: 9, program: 0, key: 0, volume: 0x80, speed: 1.0 }),
                Group::Voices => _ = sound.play_centre(&umpire, sound::Play { slot: 5, program: 0, key: 0, volume: 0x80, speed: 1.0 }),
            }
            let mut out = Vec::new();
            for _ in 0..120 {
                sound.0.lock().unwrap().render(&mut out);
            }
            out.iter().fold(0f32, |a, x| a.max(x.abs()))
        };
        for (g, group) in [Group::Music, Group::Sfx, Group::Voices].into_iter().enumerate() {
            let full = peak(group, [10; 4]);
            assert!(full > 0.01, "{group:?} sounds");
            for zero in 0..4 {
                let mut v = [10; 4];
                v[zero] = 0;
                let p = peak(group, v);
                assert_eq!(p == 0.0, zero == 0 || zero == g + 1, "{group:?} with slider {zero} at 0: peak {p}");
                if p != 0.0 {
                    assert_eq!(p, full);
                }
            }
            let half = peak(group, [5, 10, 10, 10]);
            assert!(half > full * 0.4 && half < full * 0.6, "{group:?} at half master: {half} of {full}");
        }
        assert_eq!(Volumes::parse(&Volumes([3, 0, 10, 7]).text()), Volumes([3, 0, 10, 7]));
        assert_eq!(Volumes::parse("music_volume = 99\nbogus = 1\n"), Volumes([10; 4]));
        assert_eq!(Group::of(1), Group::Voices);
        assert_eq!(Group::of(0x16), Group::Voices);
        assert_eq!(Group::of(6), Group::Sfx);
        assert_eq!(Group::of(8), Group::Music);
    }

    /// The first system sound renders: the voice keys on, sounds, and ends.
    #[test]
    fn plays_a_system_sound() {
        let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let bank = Arc::new(SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd").unwrap());
        let sound = Sound(Arc::new(Mutex::new(mix(&mut iso))));
        sound.play(&bank, 0, 0, Level { seq: [127; 2], bank: 127, pan: [0x40; 3], ..default() }, 0x1000);
        let mut out = Vec::new();
        let mut m = sound.0.lock().unwrap();
        for _ in 0..600 {
            m.render(&mut out);
        }
        let loud = out.iter().fold(0f32, |a, x| a.max(x.abs()));
        eprintln!("peak {loud}, voices left {}", m.voices.len());
        assert!(loud > 0.01 && m.voices.is_empty() && m.playing.is_empty());
    }

    /// B203: mashing a menu cue (every frame, as a held direction's repeat or fast presses) peaks no louder than
    /// one press: each cue cuts the one before.
    #[test]
    fn menu_cues_do_not_stack() {
        let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let bank = Arc::new(SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd").unwrap());
        let sound = Sound(Arc::new(Mutex::new(mix(&mut iso))));
        let peak = |presses: usize, every: usize| {
            let mut out = Vec::new();
            for f in 0..120 {
                if f % every == 0 && f / every < presses {
                    sound.play_menu(&bank, 0);
                }
                sound.0.lock().unwrap().render(&mut out);
            }
            assert!(sound.0.lock().unwrap().voices.len() <= 2);
            out.iter().fold(0f32, |a, x| a.max(x.abs()))
        };
        let one = peak(1, 1);
        for every in [1, 2, 4, 6] {
            let mashed = peak(usize::MAX, every);
            eprintln!("one {one}, every {every} frames {mashed}");
            assert!(mashed <= one * 1.01, "every {every}: {mashed} > {one}");
        }
    }

    /// B307: a cursor move right after a select lets the select cue play out under it; a select cuts both.
    #[test]
    fn cursor_move_keeps_select() {
        let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let bank = Arc::new(SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd").unwrap());
        let sound = Sound(Arc::new(Mutex::new(mix(&mut iso))));
        let mut out = Vec::new();
        let select = sound.play_menu(&bank, 2);
        sound.0.lock().unwrap().render(&mut out);
        assert!(sound.playing(select));
        let mut moves = Vec::new();
        for _ in 0..3 {
            moves.push(sound.play_menu(&bank, 0));
            sound.0.lock().unwrap().render(&mut out);
            assert!(sound.playing(select), "a cursor move cut the select cue");
        }
        assert!(moves[..2].iter().all(|&id| !sound.playing(id)), "cursor moves stacked");
        let back = sound.play_menu(&bank, 1);
        assert!(!sound.playing(select) && !sound.playing(moves[2]) && sound.playing(back));
    }

    /// A mod's `voice/<program>_<key>.wav` takes load (stereo 22.05 kHz → mono), play at the sequence's full
    /// level and end; a key past the takes wraps round them; a program without wavs is silent, or the donor's.
    #[test]
    fn plays_mod_wavs() {
        let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let dir = std::env::temp_dir().join(format!("hst-wav-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let samples: Vec<u8> = (0..2205).flat_map(|i| { let x = ((i as f32 * 0.3).sin() * 16000.0) as i16; [x, x] }).flat_map(i16::to_le_bytes).collect();
        let mut wav = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        wav.extend([16u32.to_le_bytes(), [1, 0, 2, 0], 22050u32.to_le_bytes(), (22050u32 * 4).to_le_bytes(), [4, 0, 16, 0]].concat());
        wav.extend(b"data");
        wav.extend((samples.len() as u32).to_le_bytes());
        wav.extend(&samples);
        for f in ["1_0.wav", "1_1.wav", "3_0.wav"] {
            std::fs::write(dir.join(f), &wav).unwrap();
        }
        std::fs::write(dir.join("notes.txt"), b"x").unwrap();
        let bank = Arc::new(SoundBank::wavs(&dir).unwrap());
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!((bank.wav[&1].len(), bank.wav[&3].len(), bank.wav[&1][0].0.len(), bank.wav[&1][0].1), (2, 1, 2205, 22050));
        let sound = Sound(Arc::new(Mutex::new(mix(&mut iso))));
        assert_eq!(sound.play(&bank, 2, 0, Level { seq: [127; 2], bank: 127, pan: [0x40; 3], ..default() }, 0x1000), 0);
        let id = sound.play(&bank, 1, 7, Level { seq: [127; 2], bank: 127, pan: [0x40; 3], ..default() }, 0x1000);
        let mut out = Vec::new();
        let mut frames = 0;
        while sound.playing(id) && frames < 60 {
            sound.0.lock().unwrap().render(&mut out);
            frames += 1;
        }
        let peak = out.iter().fold(0f32, |a, x| a.max(x.abs()));
        eprintln!("wav peak {peak}, {frames} frames");
        assert!(frames == 6 && peak > 0.1 && peak < 1.0);
        // with the donor's disc bank beside the wavs (`mod_match::apply`), a program the mod lacks plays the disc's
        let disc = voice_bank(&mut iso, 0, 2, 1, false).unwrap();
        let both = Arc::new(SoundBank { hd: disc.hd, bd: disc.bd, wav: bank.wav.clone() });
        assert_ne!(sound.play(&both, 2, 0, Level { seq: [127; 2], bank: 127, pan: [0x40; 3], ..default() }, 0x1000), 0);
    }

    /// Each of character 0's disc voice takes that keys on at once at full velocity and tone volume, written out as
    /// a mono wav at the rate its voice plays it, sounds sample for sample like the disc take through `Sound::play`:
    /// as recorded, at a slower play speed, and stopped part-way (the release).
    #[test]
    fn wav_from_a_disc_take_plays_like_it() {
        let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let disc = Arc::new(voice_bank(&mut iso, 0, 2, 1, false).unwrap());
        let b = Bank::parse(&disc.hd).unwrap();
        let mix = mix(&mut iso);
        let dir = std::env::temp_dir().join(format!("hst-take-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut takes = Vec::new();
        for (p, k) in (0..16).flat_map(|p| (0..8).map(move |k| (p, k))) {
            let Some(&[e]) = b.key_ons(p, k).as_deref() else { continue };
            let t = b.tone(e.set as usize, e.note).unwrap();
            if e.frame != 0 || e.velocity != 127 || t.volume() != 127 || b.set_volume(e.set as usize) != Some(127) {
                continue;
            }
            assert_eq!((t.adsr(), t.centre(), t.pan()), (VOICE_TONE.adsr(), true, 64));
            let word = (t.root() as u32) << 24 | (e.note as u32) << 16 | (t.fine() as u8 as u32) << 8 | 0x40;
            let pitch = snd::pitch(&mix.pitch, word, 0x0100_1000) as u32;
            let rate = (pitch * 48_000 + 2048) / 4096;
            assert_eq!(wav_pitch(rate, 0x1000) as u32, pitch);
            let pcm = snd::adpcm(&disc.bd, t.sample());
            let mut wav = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
            wav.extend([16u32.to_le_bytes(), [1, 0, 1, 0], rate.to_le_bytes(), (rate * 2).to_le_bytes(), [2, 0, 16, 0]].concat());
            wav.extend(b"data");
            wav.extend((pcm.len() as u32 * 2).to_le_bytes());
            wav.extend(pcm.iter().flat_map(|x| x.to_le_bytes()));
            std::fs::write(dir.join(format!("{p}_{}.wav", takes.len())), &wav).unwrap();
            takes.push((p, k, takes.len()));
        }
        let mods = Arc::new(SoundBank::wavs(&dir).unwrap());
        std::fs::remove_dir_all(&dir).ok();
        eprintln!("{} takes", takes.len());
        assert!(takes.len() >= 5);
        let level = Level { seq: [100, 60], bank: 127, pan: [0x40; 3], ..default() };
        let run = |bank: &Arc<SoundBank>, p, k, scale, stop: u32| {
            let sound = Sound(Arc::new(Mutex::new(mix_clone(&mix))));
            let id = sound.play(bank, p, k, level, scale);
            let mut out = Vec::new();
            for f in 0..600 {
                if f == stop {
                    sound.stop(id);
                }
                sound.0.lock().unwrap().render(&mut out);
            }
            assert!(!sound.playing(id));
            out
        };
        for &(p, k, w) in &takes {
            // a key wraps round the program's takes: find this take's index among program p's
            let wk = takes.iter().filter(|t| t.0 == p && t.2 < w).count();
            for (scale, stop) in [(0x1000, u32::MAX), (0x0c00, u32::MAX), (0x1000, 10)] {
                let (a, b) = (run(&disc, p, k, scale, stop), run(&mods, p, wk, scale, stop));
                assert!(a.iter().any(|&x| x != 0.0));
                assert!(a == b, "take {p}/{k} at {scale:#x}, stop {stop}: first difference at {:?}", a.iter().zip(&b).position(|(x, y)| x != y));
            }
        }
    }

    /// B111: a held match sound stands still (no key-ons, no samples) and goes on where it was when released;
    /// sounds from before the match aren't held.
    #[test]
    fn holds_match_sounds() {
        let sound = Sound::bare();
        let before = sound.endless();
        sound.begin_match();
        let in_match = sound.endless();
        let frames = || sound.0.lock().unwrap().playing.iter().map(|p| p.frame).collect::<Vec<_>>();
        let render = || sound.0.lock().unwrap().render(&mut Vec::new());
        render();
        sound.hold(true);
        render();
        assert_eq!(frames(), [2, 1]);
        sound.hold(false);
        render();
        assert_eq!(frames(), [3, 2]);
        assert!(sound.playing(before) && sound.playing(in_match));
    }

    /// B77: paused mid-flight, the lob's whistle (re-levelled every frame) stops where it is, silent, until the match's
    /// next update after the pause; its voice isn't released.
    #[test]
    fn pause_holds_the_whistle() {
        let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let sound = Sound(Arc::new(Mutex::new(mix(&mut iso))));
        let bank = court_bank(&mut iso, 10).0.unwrap();
        let at = [1.0, 2.0, 5.0];
        let id = sound.play_at(&bank, sound::FLIGHT, at);
        // the largest step between neighbouring samples of one side
        let render = |frames: usize| {
            let mut out = Vec::new();
            for _ in 0..frames {
                sound.0.lock().unwrap().render(&mut out);
            }
            out.windows(3).map(|w| (w[2] - w[0]).abs()).fold(0f32, f32::max)
        };
        render(2);
        sound.update(id, sound::FLIGHT, at);
        assert!(render(20) > 0.01, "the whistle sounds");
        sound.hold_followed();
        let held = render(240);
        assert!(held < 1e-4 && sound.playing(id), "held: steps of {held}");
        sound.update(id, sound::FLIGHT, at);
        assert!(render(20) > 0.01, "the whistle goes on");
    }

    fn mix_clone(m: &Mix) -> Mix {
        Mix { pitch: m.pitch.clone(), pan: m.pan.clone(), gain: m.gain.clone(), stereo: m.stereo.clone(), bank_volumes: m.bank_volumes.clone(), voices: Vec::new(), playing: Vec::new(), music: None, ids: 0, tags: Tags::default(), followed: Vec::new(), vol: Vol::default(), menu: [0; 2] }
    }
}
