//! Android start-up (M99): the app's external files dir (/sdcard/Android/data/<package>/files) holds the disc pack,
//! settings.txt and mods/. With no disc there yet, a start page says which disc the game needs and opens the system's
//! file picker (`HstActivity.pick`, the Storage Access Framework: Downloads, SD cards, cloud drives); the picked ISO is
//! imported into `disc.pack` as on the desktop (`disc.rs`, M13) and the app restarts into the main menu
//! (`HstActivity.restart`: the native window can only be made once per process).
//!
//! M102: also what the Android app (`tools/apk.sh`, NativeActivity) asks the Java side for, through JNI: the window's safe
//! area (system bars and the camera cutout, which an edge-to-edge window draws under) and the on-screen pad's haptic
//! tick (`play/touch_pad.rs`).
//!
//! The same dir holds `hst-log.txt`, this launch's log (stderr, panics, fatal signals and the engine's log), so a phone's
//! freeze or crash can be read without adb logcat; the launch before's is kept as `hst-log-previous.txt` (reopening the
//! app after a crash would otherwise overwrite the crash's log). Everything still goes to logcat too.

use bevy::input::keyboard::KeyboardInput;
use bevy::log::{BoxedFmtLayer, tracing_subscriber};
use bevy::prelude::*;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use jni::objects::{JObject, JValue};
use jni::{JNIEnv, JavaVM};

/// This launch's `hst-log.txt`.
static LOG: OnceLock<std::fs::File> = OnceLock::new();

/// stdout and stderr into `dir/hst-log.txt` as well as logcat (android-activity's forwarder, which they went to
/// before), fresh each launch. Call first thing in `android_main`.
pub fn log_to_files(dir: &std::path::Path) {
    let path = dir.join("hst-log.txt");
    let _ = std::fs::rename(&path, dir.join("hst-log-previous.txt"));
    let Ok(file) = std::fs::File::create(&path) else { return };
    let file = LOG.get_or_init(|| file);
    // a pipe over fds 1 and 2; a thread copies what comes through into the file and on to the old fd 2 (logcat)
    let mut fds = [0; 2];
    // SAFETY: plain pipe/dup/dup2 of fds this process owns, before any thread of ours writes to them
    unsafe {
        if libc::pipe(fds.as_mut_ptr()) != 0 {
            return;
        }
        let logcat = libc::dup(2);
        libc::dup2(fds[1], 1);
        libc::dup2(fds[1], 2);
        libc::close(fds[1]);
        let mut pipe = std::fs::File::from_raw_fd(fds[0]);
        let mut logcat = (logcat >= 0).then(|| std::fs::File::from_raw_fd(logcat));
        let _ = std::thread::Builder::new().name("hst-log".into()).spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = pipe.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let _ = (&*file).write_all(&buf[..n]);
                if let Some(l) = &mut logcat {
                    let _ = l.write_all(&buf[..n]);
                }
            }
        });
    }
    // the frame-rate log (`main.rs` log_fps, log_assets; `graphics.rs` log_draws) every 5 s: the phone's only profiler
    // SAFETY: first thing in android_main, before any thread of ours reads the environment
    unsafe { std::env::set_var("HST_FPS", "1") };
    eprintln!("Hot Shots Tennis {} (build {})", env!("CARGO_PKG_VERSION"), option_env!("HST_BUILD").unwrap_or("?"));
    // a close with no log line and no crash report says nothing: these say which kind of end it was
    unsafe {
        libc::atexit(on_exit);
        for sig in [libc::SIGABRT, libc::SIGSEGV, libc::SIGBUS, libc::SIGILL, libc::SIGTRAP, libc::SIGFPE] {
            let mut new: libc::sigaction = std::mem::zeroed();
            new.sa_sigaction = on_signal as *const () as libc::sighandler_t;
            new.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
            libc::sigaction(sig, &new, &raw mut OLD[sig as usize]);
        }
    }
}

extern "C" fn on_exit() {
    eprintln!("exit() called");
}

/// The handlers before ours (debuggerd's, which writes the tombstone), by signal number.
static mut OLD: [libc::sigaction; 32] = unsafe { std::mem::zeroed() };

extern "C" fn on_signal(sig: libc::c_int, info: *mut libc::siginfo_t, ctx: *mut libc::c_void) {
    // only async-signal-safe calls here: write straight to the file (the pipe's thread may never run again), then
    // hand the signal on to the handler before ours
    let msg: &[u8] = match sig {
        libc::SIGABRT => b"fatal signal: SIGABRT (abort)\n",
        libc::SIGSEGV => b"fatal signal: SIGSEGV (bad memory access)\n",
        libc::SIGBUS => b"fatal signal: SIGBUS (bad memory access)\n",
        libc::SIGILL => b"fatal signal: SIGILL\n",
        libc::SIGTRAP => b"fatal signal: SIGTRAP\n",
        _ => b"fatal signal: SIGFPE\n",
    };
    unsafe {
        if let Some(f) = LOG.get() {
            libc::write(f.as_raw_fd(), msg.as_ptr().cast(), msg.len());
        }
        let old = &*(&raw const OLD[sig as usize]);
        if old.sa_flags & libc::SA_SIGINFO != 0 && old.sa_sigaction > libc::SIG_IGN {
            let h: extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void) = std::mem::transmute(old.sa_sigaction);
            h(sig, info, ctx);
        } else if old.sa_sigaction > libc::SIG_IGN {
            let h: extern "C" fn(libc::c_int) = std::mem::transmute(old.sa_sigaction);
            h(sig);
        }
        // nobody's (or a returning) handler: die of it
        libc::signal(sig, libc::SIG_DFL);
        libc::raise(sig);
    }
}

/// `LogPlugin::fmt_layer`: the engine's log as plain text into `hst-log.txt`, in place of bevy's coloured one on
/// stderr (logcat gets the engine's log on its own).
pub fn log_layer(_: &mut App) -> Option<BoxedFmtLayer> {
    let file = LOG.get()?;
    Some(Box::new(tracing_subscriber::fmt::Layer::default().with_ansi(false).with_writer(move || file)))
}

/// The first frame, a heartbeat every 5 s, quits asked for and lifecycle changes (pause, resume), in the log.
pub fn heartbeat(time: Res<Time<Real>>, mut frames: Local<u64>, mut last: Local<f32>, mut exits: MessageReader<AppExit>, mut life: MessageReader<bevy::window::AppLifecycle>) {
    *frames += 1;
    let t = time.elapsed_secs();
    if *frames == 1 || t - *last >= 5.0 {
        *last = t;
        eprintln!("frame {} at {t:.1}s", *frames);
    }
    for e in exits.read() {
        eprintln!("AppExit {e:?} at frame {}", *frames);
    }
    for l in life.read() {
        eprintln!("lifecycle {l:?} at {t:.1}s");
    }
}

/// A method of the running `HstActivity` (no arguments, returns `int` or `void`: 0), on the calling thread (attached to
/// the VM for good). `None` if the call failed.
fn call(name: &str, sig: &str) -> Option<i32> {
    let app = bevy::android::ANDROID_APP.get()?;
    // SAFETY: the VM and the activity outlive the process's native code; `JObject` doesn't free what it wraps
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
    let mut env = vm.attach_current_thread_permanently().ok()?;
    let activity = unsafe { jni::objects::JObject::from_raw(app.activity_as_ptr().cast()) };
    let r = env.call_method(&activity, name, sig, &[]);
    if r.is_err() {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    r.ok().map(|v| v.i().unwrap_or(0))
}

#[derive(Resource)]
struct Start {
    pack: PathBuf,
    stage: Stage,
}

enum Stage {
    /// Waiting for a tap; the last try's error, if any.
    Idle(Option<String>),
    /// The system's picker is up.
    Picking,
    /// Copying the disc: per mille done, and the result once the thread is through.
    Importing(Arc<AtomicU32>, Arc<Mutex<Option<Result<(), String>>>>),
    /// Imported; restarting.
    Done,
}

#[derive(Component)]
struct Status;

const INTRO: &str = "This remaster plays from your own copy of the original game:\n\
                     Hot Shots Tennis (USA), SCUS-97610, version 1.00.\n\n\
                     Choose its disc image (.iso) on this device: Downloads, an SD card or a cloud drive. \
                     The game copies the part it needs (about 170 MB); after that you can delete the ISO.";

/// No disc in `dir` yet: the start page, until the import restarts the app (see the module doc).
pub fn no_disc(dir: PathBuf) -> ! {
    info!("no disc in {}", dir.display());
    App::new()
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin { fmt_layer: log_layer, ..default() }))
        .insert_resource(ClearColor(Color::srgb(0.05, 0.12, 0.2)))
        .insert_resource(Start { pack: dir.join("disc.pack"), stage: Stage::Idle(None) })
        .add_systems(Startup, page)
        .add_systems(Update, (tap, poll, show).chain())
        .run();
    std::process::exit(0)
}

/// Sizes in `vmin` so the page lays out the same at any aspect ratio and resolution.
fn page(mut commands: Commands) {
    commands.spawn(Camera2d);
    let font = |size: f32| TextFont { font_size: FontSize::VMin(size), ..default() };
    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: vmin(3),
            ..default()
        })
        .with_children(|p| {
            p.spawn((Text::new("Hot Shots Tennis"), font(7.0), TextColor(Color::srgb(1.0, 0.85, 0.3))));
            p.spawn((Node { max_width: vmin(140), ..default() }, Text::new(INTRO), font(3.6), TextLayout::justify(Justify::Center)));
            p.spawn((
                Button,
                Node { padding: UiRect::axes(vmin(5), vmin(2)), margin: UiRect::top(vmin(2)), border_radius: BorderRadius::all(vmin(2)), ..default() },
                BackgroundColor(Color::srgb(0.95, 0.55, 0.1)),
            ))
            .with_child((Text::new("Choose disc image"), font(4.5), TextColor(Color::BLACK)));
            p.spawn((Status, Node { max_width: vmin(140), ..default() }, Text::new(""), font(3.2), TextLayout::justify(Justify::Center), TextColor(Color::srgb(0.75, 0.85, 1.0))));
        });
}

/// A tap anywhere, a click or any button (a pad's arrive as key presses) opens the picker.
fn tap(mut start: ResMut<Start>, touches: Res<Touches>, mouse: Res<ButtonInput<MouseButton>>, mut keys: MessageReader<KeyboardInput>) {
    let key = keys.read().any(|k| k.state.is_pressed() && !k.repeat && k.key_code != KeyCode::BrowserBack);
    if !matches!(start.stage, Stage::Idle(_)) || !(touches.any_just_released() || mouse.any_just_released([MouseButton::Left]) || key) {
        return;
    }
    if call("pick", "()V").is_some() {
        start.stage = Stage::Picking;
    } else {
        start.stage = Stage::Idle(Some("The file picker didn't open.".into()));
    }
}

/// The picker's answer → the import on its own thread → the restart.
fn poll(mut start: ResMut<Start>) {
    match &start.stage {
        Stage::Picking => match call("taken", "()I") {
            Some(-2) | None => {}
            Some(fd) if fd < 0 => start.stage = Stage::Idle(None),
            Some(fd) => {
                // SAFETY: `detachFd` handed this descriptor over to us
                let file = unsafe { std::fs::File::from_raw_fd(fd) };
                let (done, result) = (Arc::new(AtomicU32::new(0)), Arc::new(Mutex::new(None)));
                let (d, r, pack) = (done.clone(), result.clone(), start.pack.clone());
                std::thread::spawn(move || {
                    let out = import(file, &pack, &d);
                    *r.lock().unwrap() = Some(out);
                });
                start.stage = Stage::Importing(done, result);
            }
        },
        Stage::Importing(_, result) => {
            let Some(out) = result.lock().unwrap().take() else { return };
            start.stage = match out {
                Ok(()) => {
                    info!("disc imported into {}: restarting", start.pack.display());
                    call("restart", "()V");
                    Stage::Done
                }
                Err(e) => Stage::Idle(Some(e)),
            };
        }
        _ => {}
    }
}

fn import(file: std::fs::File, pack: &std::path::Path, done: &AtomicU32) -> Result<(), String> {
    let _ = std::fs::create_dir_all(pack.parent().unwrap());
    let mut disc = hst_data::iso::Iso::from_source(Box::new(file)).map_err(|e| {
        info!("not a disc image: {e}");
        "That file isn't a disc image. Choose the Hot Shots Tennis (USA) ISO.".to_string()
    })?;
    let hash = disc
        .write_pack(pack, |i, n| done.store((i * 1000 / n.max(1)) as u32, Ordering::Relaxed))
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::InvalidData => format!("That's not the disc this game needs ({e}). It needs Hot Shots Tennis (USA), SCUS-97610, version 1.00."),
            _ => format!("Couldn't copy the disc: {e}"),
        })?;
    info!("disc pack {hash:016x}");
    Ok(())
}

fn show(start: Res<Start>, mut status: Query<&mut Text, With<Status>>) {
    let text = match &start.stage {
        Stage::Idle(None) => "Tap anywhere or press a button to choose.".to_string(),
        Stage::Idle(Some(e)) => format!("{e}\n\nTap to choose again."),
        Stage::Picking => "Waiting for the file picker…".to_string(),
        Stage::Importing(done, _) => format!("Copying the disc: {}%", done.load(Ordering::Relaxed) / 10),
        Stage::Done => "Done. Starting the game… (if it doesn't, close and reopen the app)".to_string(),
    };
    for mut t in &mut status {
        if t.0 != text {
            t.0 = text.clone();
        }
    }
}

// M102: safe area and haptics


/// The activity's VM, attached to the calling thread for good (the pad's systems run on a few pool threads).
fn with_env<T>(f: impl FnOnce(&mut JNIEnv, &JObject) -> jni::errors::Result<T>) -> Option<T> {
    static VM: OnceLock<Option<JavaVM>> = OnceLock::new();
    let app = bevy::android::ANDROID_APP.get()?;
    // SAFETY: the activity's VM pointer, valid for the process
    let vm = VM.get_or_init(|| unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()).as_ref()?;
    let mut env = vm.attach_current_thread_permanently().ok()?;
    // SAFETY: the NativeActivity's global reference, alive as long as the activity
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
    // a frame for the local references: a permanently attached thread never returns to Java to free them
    let out = env.with_local_frame(16, |env| f(env, &activity));
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    out.map_err(|e| warn!("android: JNI call failed: {e}")).ok()
}

fn decor<'a>(env: &mut JNIEnv<'a>, activity: &JObject) -> jni::errors::Result<JObject<'a>> {
    let window = env.call_method(activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
    env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?.l()
}

/// The window's insets, left, top, right, bottom, in physical px: the visible system bars and the display cutout
/// (0 where the window doesn't reach under them).
pub fn safe_area() -> Option<[f32; 4]> {
    with_env(|env, activity| {
        let decor = decor(env, activity)?;
        let insets = env.call_method(&decor, "getRootWindowInsets", "()Landroid/view/WindowInsets;", &[])?.l()?;
        if insets.is_null() {
            return Ok([0.0; 4]);
        }
        let sdk = env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?.i()?;
        let side = |env: &mut JNIEnv, o: &JObject, name: &str| env.get_field(o, name, "I").and_then(|v| v.i());
        if sdk >= 30 {
            let bars = env.call_static_method("android/view/WindowInsets$Type", "systemBars", "()I", &[])?.i()?;
            let cutout = env.call_static_method("android/view/WindowInsets$Type", "displayCutout", "()I", &[])?.i()?;
            let i = env.call_method(&insets, "getInsets", "(I)Landroid/graphics/Insets;", &[JValue::Int(bars | cutout)])?.l()?;
            Ok([side(env, &i, "left")?, side(env, &i, "top")?, side(env, &i, "right")?, side(env, &i, "bottom")?].map(|v| v as f32))
        } else {
            // before 30: the system windows' insets, which take in the cutout once the window lays out into it
            let mut v = [0.0; 4];
            for (v, m) in v.iter_mut().zip(["getSystemWindowInsetLeft", "getSystemWindowInsetTop", "getSystemWindowInsetRight", "getSystemWindowInsetBottom"]) {
                *v = env.call_method(&insets, m, "()I", &[])?.i()? as f32;
            }
            Ok(v)
        }
    })
}

/// M102: the light haptic tick of an on-screen pad press, as a key's (the phone's own touch-feedback setting holds too).
pub fn haptic() {
    with_env(|env, activity| {
        let decor = decor(env, activity)?;
        // HapticFeedbackConstants.VIRTUAL_KEY
        env.call_method(&decor, "performHapticFeedback", "(I)Z", &[JValue::Int(1)]).map(drop)
    });
}
