//! iOS start-up (M91): the app's Documents folder (Files app → On My iPhone → Hot Shots) holds the ISO, settings.txt,
//! mods/ and `hst-log.txt`, this launch's log (stderr, panics and the engine's log; iOS shows nothing else without a
//! Mac). Writing the log also makes iOS list the folder in Files, which it only does once there's a file in it.

use bevy::log::{BoxedLayer, tracing_subscriber};
use bevy::prelude::*;
use std::os::fd::AsRawFd;

/// stderr into `Documents/hst-log.txt`, fresh each launch. Call first thing in `main`.
pub fn log_to_documents(docs: &std::path::Path) {
    let _ = std::fs::create_dir_all(docs);
    if let Ok(f) = std::fs::File::create(docs.join("hst-log.txt")) {
        // SAFETY: plain dup2 of an open file over fd 2
        unsafe { libc::dup2(f.as_raw_fd(), 2) };
    }
    eprintln!("Hot Shots Tennis {} (build {})", env!("CARGO_PKG_VERSION"), option_env!("HST_BUILD").unwrap_or("?"));
    // a close with no log line and no crash report says nothing: these say which kind of end it was
    unsafe {
        libc::atexit(on_exit);
        for sig in [libc::SIGABRT, libc::SIGSEGV, libc::SIGBUS, libc::SIGILL, libc::SIGTRAP, libc::SIGFPE] {
            libc::signal(sig, on_signal as *const () as libc::sighandler_t);
        }
        NSSetUncaughtExceptionHandler(on_exception);
    }
}

extern "C" fn on_exit() {
    eprintln!("exit() called");
}

extern "C" fn on_signal(sig: libc::c_int) {
    // only async-signal-safe calls here: write, then die of the same signal
    let msg: &[u8] = match sig {
        libc::SIGABRT => b"fatal signal: SIGABRT (abort)\n",
        libc::SIGSEGV => b"fatal signal: SIGSEGV (bad memory access)\n",
        libc::SIGBUS => b"fatal signal: SIGBUS (bad memory access)\n",
        libc::SIGILL => b"fatal signal: SIGILL\n",
        libc::SIGTRAP => b"fatal signal: SIGTRAP\n",
        _ => b"fatal signal: SIGFPE\n",
    };
    unsafe {
        libc::write(2, msg.as_ptr().cast(), msg.len());
        libc::signal(sig, libc::SIG_DFL);
        libc::raise(sig);
    }
}

// UIKit failures are Objective-C exceptions: their name and reason, before the abort
unsafe extern "C" {
    fn NSSetUncaughtExceptionHandler(handler: extern "C" fn(*mut std::ffi::c_void));
    fn sel_registerName(name: *const std::ffi::c_char) -> *mut std::ffi::c_void;
    fn objc_msgSend();
}

extern "C" fn on_exception(e: *mut std::ffi::c_void) {
    let text = |obj: *mut std::ffi::c_void, sel: &std::ffi::CStr| unsafe {
        let send: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> *mut std::ffi::c_void = std::mem::transmute(objc_msgSend as *const ());
        let s = send(obj, sel_registerName(sel.as_ptr()));
        let utf8: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> *const std::ffi::c_char = std::mem::transmute(objc_msgSend as *const ());
        if s.is_null() { "?".into() } else { std::ffi::CStr::from_ptr(utf8(s, sel_registerName(c"UTF8String".as_ptr()))).to_string_lossy().into_owned() }
    };
    eprintln!("Objective-C exception {}: {}", text(e, c"name"), text(e, c"reason"));
}

/// The first frame, a heartbeat every 5 s, quits asked for and iOS lifecycle changes, in the log.
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

/// `LogPlugin::custom_layer`: the engine's log as text on stderr (bevy only sends it to os_log on iOS).
pub fn log_layer(_: &mut App) -> Option<BoxedLayer> {
    Some(Box::new(tracing_subscriber::fmt::Layer::default().with_ansi(false).with_writer(std::io::stderr)))
}

/// No disc yet: a page that says where the ISO goes, instead of a launch that closes on a black screen.
pub fn no_disc() -> ! {
    eprintln!("no disc image in Documents");
    App::new()
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin { custom_layer: log_layer, ..default() }))
        .insert_resource(ClearColor(Color::srgb(0.05, 0.12, 0.2)))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
            commands
                .spawn(Node { width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() })
                .with_child((
                    Node { max_width: percent(85), ..default() },
                    Text::new(
                        "Hot Shots Tennis needs your disc image.\n\n\
                         In the Files app, open On My iPhone → Hot Shots\n\
                         (the folder with this app's icon), put your\n\
                         Hot Shots Tennis (USA) ISO in it, then close and reopen this app.",
                    ),
                    TextFont { font_size: FontSize::VMin(3.5), ..default() },
                ));
        })
        .run();
    std::process::exit(0)
}
