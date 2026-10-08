//! The game's sound: SPU2 voices (`snd::Voice`) mixed into one 48 kHz stereo stream, playing bank sequences the
//! way the sound driver does — one sequence step a frame (800 samples), each key-on resolved to its tone, pitch and
//! L/R volume. `--sound <archive> <bank.hd> <program> <key>` auditions one sound. The music is a MIDI file on its
//! own bank, stepped a frame at a time by `snd::Sequencer`, each note-on resolved through the bank's programs.

use crate::Args;
use bevy::audio::{AddAudioSource, ChannelCount, Decodable, SampleRate, Source};
use bevy::prelude::*;
use hst_data::{exe, iso::Iso, snd::{self, Bank, KeyOn, Level, Voice}, xb::Archive};
use hst_sim::sound;
use std::sync::{Arc, Mutex};

/// Output samples per 60 Hz frame.
const FRAME: usize = 48_000 / 60;

/// A sound bank: `.hd` header and `.bd` samples.
pub struct SoundBank {
    pub hd: Vec<u8>,
    pub bd: Vec<u8>,
}

impl SoundBank {
    /// `hd` names the header inside the disc archive `xb`; the samples sit beside it as `.bd`.
    pub fn load(iso: &mut Iso, xb: &str, hd: &str) -> Option<Self> {
        let data = iso.read(xb).ok()?;
        let arc = Archive::parse(&data).ok()?;
        let read = |name: &str| arc.read(arc.entries.iter().find(|e| e.name.replace('\\', "/").ends_with(name))?).ok();
        Some(Self { hd: read(hd)?, bd: read(&hd.replace(".hd", ".bd"))? })
    }

    /// A BGM: bank `data/sound/BGM/<dir>/<name>.hd` and the MIDI file beside it in `SND/BGM/<NAME>.XB`.
    pub fn music(iso: &mut Iso, dir: &str, name: &str) -> Option<(Self, Vec<u8>)> {
        let xb = format!("SND/BGM/{}.XB", name.to_uppercase());
        Some((Self::load(iso, &xb, &format!("data/sound/BGM/{dir}/{name}.hd"))?, midi(iso, &xb)?))
    }
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
    /// Each voice with its bank, play id, note, pitch word and level (for `Sound::update`).
    voices: Vec<(Voice, Arc<SoundBank>, u64, u8, u32, Level)>,
    playing: Vec<Playing>,
    music: Option<Music>,
    ids: u64,
}

impl Mix {
    /// The play call's part of the level for `p` at `pos`.
    fn level(&self, p: sound::Play, pos: [f32; 3]) -> Level {
        let (angle, dist) = sound::place(pos);
        let seq = sound::stereo(sound::falloff(p.volume, dist), angle, &self.stereo).map(|x| x as u32);
        Level { seq, bank: self.bank_volumes[p.slot as usize], pan: [0x40; 3], ..default() }
    }

    /// One frame: the sequences' events due now, then `FRAME` stereo samples.
    fn render(&mut self, out: &mut Vec<f32>) {
        let mut playing = std::mem::take(&mut self.playing);
        for p in &mut playing {
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
        for _ in 0..FRAME {
            let mut s = [0; 2];
            for (v, b, ..) in &mut self.voices {
                let [l, r] = v.tick(&b.bd);
                s = [s[0] + l, s[1] + r];
            }
            // ponytail: dry voices only — no core/master volume or reverb
            out.extend(s.map(|x| x.clamp(-0x8000, 0x7fff) as f32 / 32768.0));
        }
        self.voices.retain(|v| v.0.phase != 0);
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
        let Some(events) = Bank::parse(&bank.hd).ok().and_then(|b| b.key_ons(program, key)) else { return 0 };
        let mut m = self.0.lock().unwrap();
        m.ids += 1;
        let id = m.ids;
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
        m.music = Some(Music { bank: bank.clone(), seq, channels: [[0, 100, 0x40, 127]; 16], volume, id });
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
        for q in m.playing.iter_mut().filter(|q| q.id == id) {
            (q.level.seq, q.scale) = (seq, scale);
        }
        for (v, .., word, level) in m.voices.iter_mut().filter(|v| v.2 == id) {
            level.seq = seq;
            v.volume = level.volume(&m.pan);
            v.pitch = snd::pitch(&m.pitch, *word, 0x0100_0000 | scale);
        }
    }

    /// A non-positional play (the umpire's voice): centred, sequence volume `p.volume`.
    /// ponytail: the library's non-positional level is taken as the positional one straight ahead with no falloff
    pub fn play_centre(&self, bank: &Arc<SoundBank>, p: sound::Play) -> u64 {
        let bank_volume = self.0.lock().unwrap().bank_volumes[p.slot as usize];
        let level = Level { seq: [p.volume as u32; 2], bank: bank_volume, pan: [0x40; 3], ..default() };
        self.play(bank, p.program as usize, p.key as usize, level, 0x1000)
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
        self.play(bank, p.program as usize, p.key as usize, level, sound::speed_word(p.speed))
    }

    /// The play `id` (a `play_toward` one) turned to `angle`: its voices re-panned at sequence volume `volume`.
    pub fn turn(&self, id: u64, volume: i32, angle: i32) {
        let mut m = self.0.lock().unwrap();
        let m = &mut *m;
        let seq = sound::stereo(volume, angle, &m.stereo).map(|x| x as u32);
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
            let seq = sound::stereo(p.volume, angle, &m.stereo).map(|x| x as u32);
            Level { seq, bank: m.bank_volumes[p.slot as usize], pan: [0x40; 3], ..default() }
        };
        self.play(bank, p.program as usize, p.key as usize, level, sound::speed_word(p.speed))
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
    let court = SoundBank::load(&mut iso, &format!("SND/COURT/C_SND{n:02}A.XB0"), &format!("data/sound/SE/court/co_se{n:02}.hd"));
    commands.insert_resource(CourtBank(court.map(Arc::new)));
    commands.insert_resource(sound);
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

/// Character `c`'s voice bank for a game of `players`: the singles (`sgv`) or doubles (`dvv`) set, variant a or b.
pub fn voice_bank(iso: &mut Iso, c: usize, players: usize, b: bool) -> Option<SoundBank> {
    let (n, set, tag) = if players < 3 { (0, "SGL", "sgv") } else { (4, "DBL", "dvv") };
    let v = if b { 'b' } else { 'a' };
    SoundBank::load(iso, &format!("SND/VOICE/PC/PC{c:02}VCE{}.XB0", n + b as usize), &format!("data/sound/VOICE/{set}/{tag}_vc{c:02}{v}.hd"))
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
    Mix { pitch, pan, gain, stereo, bank_volumes, voices: Vec::new(), playing: Vec::new(), music: None, ids: 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every character's voice banks (0..13, singles and doubles, variants a and b) load from the disc.
    #[test]
    fn loads_voice_banks() {
        let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let missing: Vec<_> = (0..14)
            .flat_map(|c| [(c, 2, false), (c, 2, true), (c, 4, false), (c, 4, true)])
            .filter(|&(c, players, b)| voice_bank(&mut iso, c, players, b).is_none())
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
        sound.music(&Arc::new(bank), &mid, 55);
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
}
