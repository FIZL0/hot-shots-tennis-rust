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
                         1. Open the Files app → Browse → On My iPhone → Hot Shots\n   (the folder with this app's icon, not one you make yourself).\n\
                         2. Put your Hot Shots Tennis (USA) ISO (SCUS-97610) in it.\n\
                         3. Close this app (swipe it away) and open it again.",
                    ),
                    TextFont { font_size: FontSize::VMin(4.5), ..default() },
                ));
        })
        .run();
    std::process::exit(0)
}
