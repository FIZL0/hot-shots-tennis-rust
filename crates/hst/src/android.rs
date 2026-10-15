//! Android start-up (M99): the app's external files dir (/sdcard/Android/data/<package>/files) holds the disc pack,
//! settings.txt and mods/. With no disc there yet, a start page says which disc the game needs and opens the system's
//! file picker (`HstActivity.pick`, the Storage Access Framework: Downloads, SD cards, cloud drives); the picked ISO is
//! imported into `disc.pack` as on the desktop (`disc.rs`, M13) and the app restarts into the main menu
//! (`HstActivity.restart`: the native window can only be made once per process).
//!
//! M102: also what the Android app (`tools/apk.sh`, NativeActivity) asks the Java side for, through JNI: the window's safe
//! area (system bars and the camera cutout, which an edge-to-edge window draws under) and the on-screen pad's haptic
//! tick (`play/touch_pad.rs`).

use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use jni::objects::{JObject, JValue};
use jni::{JNIEnv, JavaVM};
use std::sync::OnceLock;

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
        .add_plugins(DefaultPlugins)
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
