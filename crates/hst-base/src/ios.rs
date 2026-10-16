//! iOS start-up (M91): the app's Documents folder (Files app → On My iPhone > OpenTennis) holds the ISO, settings.txt,
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
    // the frame-rate log (`main.rs` log_fps, log_assets; `graphics.rs` log_draws) every 5 s: the phone's only profiler
    crate::FPS_LOG.store(true, std::sync::atomic::Ordering::Relaxed);
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
    fn objc_getClass(name: *const std::ffi::c_char) -> *mut std::ffi::c_void;
}

/// The process's CPU time (all threads) in ms, for log_fps's ms/frame (iOS has no /proc).
pub fn cpu_ms() -> f32 {
    let mut u: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut u) };
    let ms = |t: libc::timeval| t.tv_sec as f32 * 1e3 + t.tv_usec as f32 / 1e3;
    ms(u.ru_utime) + ms(u.ru_stime)
}

/// NSProcessInfo's thermal state: the phone slows the chip from "serious" on.
pub fn thermal() -> &'static str {
    unsafe {
        let obj: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> *mut std::ffi::c_void = std::mem::transmute(objc_msgSend as *const ());
        let int: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> isize = std::mem::transmute(objc_msgSend as *const ());
        let info = obj(objc_getClass(c"NSProcessInfo".as_ptr()), sel_registerName(c"processInfo".as_ptr()));
        ["nominal", "fair", "serious", "critical"].get(int(info, sel_registerName(c"thermalState".as_ptr())) as usize).copied().unwrap_or("?")
    }
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
        // fullscreen like the game: a default (1280×720) window lays out bigger than the phone's screen
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin { custom_layer: log_layer, ..default() }).set(WindowPlugin {
            primary_window: Some(Window { mode: bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Primary), ..default() }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.05, 0.12, 0.2)))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
            commands
                .spawn(Node { width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() })
                .with_child((
                    Node { max_width: percent(85), ..default() },
                    Text::new(
                        "Hot Shots Tennis needs your disc image.\n\n\
                         In the Files app, open On My iPhone > OpenTennis\n\
                         (the folder with this app's icon), put your\n\
                         Hot Shots Tennis (USA) ISO in it, then close and reopen this app.",
                    ),
                    TextFont { font_size: FontSize::VMin(3.5), ..default() },
                ));
        })
        .run();
    std::process::exit(0)
}

// UIKit only on the main thread: the on-screen pad's calls go through its dispatch queue
unsafe extern "C" {
    static _dispatch_main_q: std::ffi::c_void;
    fn dispatch_async_f(queue: *const std::ffi::c_void, context: *mut std::ffi::c_void, work: extern "C" fn(*mut std::ffi::c_void));
}

/// M91a1: the light haptic tick of an on-screen pad press (the Vibration setting; `play/touch_pad.rs`).
pub fn haptic() {
    // SAFETY: the main queue lives for the process; `tick` takes no context
    unsafe { dispatch_async_f(&raw const _dispatch_main_q, std::ptr::null_mut(), tick) };
}

extern "C" fn tick(_: *mut std::ffi::c_void) {
    use std::sync::atomic::{AtomicPtr, Ordering};
    // one generator, made on first use and kept (main thread only)
    static GENERATOR: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());
    unsafe {
        let obj: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> *mut std::ffi::c_void = std::mem::transmute(objc_msgSend as *const ());
        let with_style: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, isize) -> *mut std::ffi::c_void = std::mem::transmute(objc_msgSend as *const ());
        let mut g = GENERATOR.load(Ordering::Relaxed);
        if g.is_null() {
            let class = objc_getClass(c"UIImpactFeedbackGenerator".as_ptr());
            if class.is_null() {
                return;
            }
            // UIImpactFeedbackStyleLight
            g = with_style(obj(class, sel_registerName(c"alloc".as_ptr())), sel_registerName(c"initWithStyle:".as_ptr()), 0);
            GENERATOR.store(g, Ordering::Relaxed);
        }
        if !g.is_null() {
            obj(g, sel_registerName(c"impactOccurred".as_ptr()));
        }
    }
}

/// UIKit's `UIEdgeInsets`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Insets {
    top: f64,
    left: f64,
    bottom: f64,
    right: f64,
}

/// The window's safe-area insets (left, top, right, bottom) in points, the window's logical px: the notch or Dynamic
/// Island's side and the home indicator's strip. As last read: each call asks the main thread for them anew.
pub fn safe_area(window: &bevy::window::RawHandleWrapper) -> [f32; 4] {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SAFE: [AtomicU32; 4] = [const { AtomicU32::new(0) }; 4];
    extern "C" fn read(view: *mut std::ffi::c_void) {
        unsafe {
            let insets: extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> Insets = std::mem::transmute(objc_msgSend as *const ());
            let i = insets(view, sel_registerName(c"safeAreaInsets".as_ptr()));
            for (a, v) in SAFE.iter().zip([i.left, i.top, i.right, i.bottom]) {
                a.store((v as f32).to_bits(), Ordering::Relaxed);
            }
        }
    }
    if let raw_window_handle::RawWindowHandle::UiKit(h) = window.get_window_handle() {
        // SAFETY: the view lives as long as the window, which outlives this frame's request
        unsafe { dispatch_async_f(&raw const _dispatch_main_q, h.ui_view.as_ptr(), read) };
    }
    SAFE.each_ref().map(|a| f32::from_bits(a.load(Ordering::Relaxed)))
}

/// P49: the display's refresh rate for the frame limit (`graphics.rs`; 0 = uncapped). A ProMotion iPhone keeps an
/// app's CAMetalLayer at 60 Hz unless something asks for more (Info.plist's CADisableMinimumFrameDurationOnPhone only
/// allows it), and wgpu's Metal surface just presents with no display link. So a display link of our own on the main
/// run loop asks for 80–120 Hz above a 60 cap and for the cap itself at or below it, so a 60 cap stays on 60 Hz vsyncs.
pub fn frame_rate(fps: u32) {
    // SAFETY: the main queue lives for the process; the context is the fps, not a pointer
    unsafe { dispatch_async_f(&raw const _dispatch_main_q, fps as usize as *mut std::ffi::c_void, set_frame_rate) };
}

objc2::define_class!(
    /// The display link's target: nothing to do on its ticks, it's only there for the rate it asks for.
    #[unsafe(super(objc2::runtime::NSObject))]
    #[name = "HstFrameRate"]
    struct FrameRate;

    impl FrameRate {
        #[unsafe(method(tick:))]
        fn tick(&self, _link: &objc2::runtime::AnyObject) {}
    }
);

extern "C" fn set_frame_rate(fps: *mut std::ffi::c_void) {
    use objc2::ClassType;
    use objc2_quartz_core::{CADisplayLink, CAFrameRateRange};
    thread_local! {
        // main thread only: made on first use, kept
        static LINK: std::cell::OnceCell<objc2::rc::Retained<CADisplayLink>> = const { std::cell::OnceCell::new() };
    }
    let fps = fps as usize as f32;
    let range = if fps == 0.0 || fps > 60.0 { CAFrameRateRange { minimum: 80.0, maximum: 120.0, preferred: 120.0 } } else { CAFrameRateRange { minimum: fps, maximum: fps, preferred: fps } };
    LINK.with(|l| {
        let link = l.get_or_init(|| unsafe {
            let target: objc2::rc::Retained<FrameRate> = objc2::msg_send![FrameRate::class(), new];
            let link = CADisplayLink::displayLinkWithTarget_selector(&target, objc2::sel!(tick:));
            link.addToRunLoop_forMode(&objc2_foundation::NSRunLoop::mainRunLoop(), objc2_foundation::NSRunLoopCommonModes);
            link
        });
        link.setPreferredFrameRateRange(range);
    });
    eprintln!("display link: {}–{} Hz, preferred {}", range.minimum, range.maximum, range.preferred);
}
