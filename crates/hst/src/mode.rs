//! Modes of one app: the main menu and a match. A mode is a set of plugins built into fresh schedules when it starts
//! (fresh resources and `Local`s, its Startup run once) and torn down when the next one starts: its entities and this
//! crate's resources it added go. The window, the pads, the audio device, the GPU and loaded textures stay up. A mode
//! builds without the render app, so Bevy plugins with a render side (materials) go in the base app.
//! A mode with a way back (the match → the menu) turns its `AppExit` into that switch while the window is open.
//! A switch puts up a loading screen first (on screen through the next mode's blocking Startup) and holds it, game
//! time paused, until the next mode's render pipelines are built and its uploads are done; the BGM stops with the old mode. Leaving a running
//! mode, its last frame zooms up about the screen centre while it fades out over the loading screen first, as the
//! original's menu → match handoff looks (similar, not frame-matched: user 2026-10-11).

use bevy::ecs::component::ComponentId;
use bevy::ecs::message::Messages;
use bevy::ecs::resource::IsResource;
use bevy::ecs::schedule::{ScheduleLabel, Schedules};
use bevy::input::gamepad::Gamepad;
use bevy::prelude::*;
use bevy::window::{Monitor, PrimaryWindow};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

type Build = Box<dyn FnOnce(&mut App) + Send + Sync>;
type Cover = Box<dyn FnOnce(&mut World, Entity) + Send + Sync>;

pub struct Mode {
    build: Build,
    back: Option<Box<Mode>>,
    cover: Option<Cover>,
}

impl Mode {
    pub fn new(build: impl FnOnce(&mut App) + Send + Sync + 'static) -> Self {
        Mode { build: Box::new(build), back: None, cover: None }
    }

    /// This mode with `more` built on top.
    pub fn and(self, more: impl FnOnce(&mut App) + Send + Sync + 'static) -> Self {
        let build = self.build;
        Mode {
            build: Box::new(move |app| {
                build(app);
                more(app);
            }),
            back: self.back,
            cover: self.cover,
        }
    }

    /// Where this mode's `AppExit` goes instead of closing the app.
    pub fn back(mut self, to: Mode) -> Self {
        self.back = Some(Box::new(to));
        self
    }

    /// What this mode's loading screen shows on its black cover (the match's hint screen, `loading.rs`).
    pub fn cover(mut self, f: impl FnOnce(&mut World, Entity) + Send + Sync + 'static) -> Self {
        self.cover = Some(Box::new(f));
        self
    }
}

/// The mode to switch to at the start of the next frame.
#[derive(Resource, Default)]
pub struct Next(pub Option<Mode>);

/// In with an `AppExit`: a real quit (an unattended run's `--shot` or `HST_MASH` done), not the mode's way back.
#[derive(Resource)]
pub struct Quit;

/// Whether the running mode's next fixed tick may run (online play waits for the peers' pads).
#[derive(Resource)]
pub struct Gate(pub fn(&mut World) -> bool);

/// The running mode's schedules.
#[derive(Resource)]
struct Running {
    s: Schedules,
    back: Option<Box<Mode>>,
    /// Its way back taken: the switch to it is pending.
    leaving: bool,
}

/// What the app had before the first mode: kept through every switch.
#[derive(Resource)]
struct Base {
    entities: HashSet<Entity>,
    resources: HashSet<ComponentId>,
    clear: ClearColor,
}

/// The loading screen: `up` frames shown before the switch, then `after` frames since it, `idle` of them in a row
/// with no pipeline compiling and the render side settled ([`Unsettled`]).
#[derive(Component, Default)]
pub(crate) struct Loading {
    up: u32,
    built: bool,
    after: u32,
    idle: u32,
}

/// The running mode's last frame asked for (a screenshot), `waited` frames ago.
#[derive(Component)]
struct Leaving {
    waited: u32,
}

/// That frame over the loading screen, `t` seconds into its zoom and fade.
#[derive(Component)]
struct Snapshot {
    t: f32,
}

/// The zoom and fade's length (the original's ~30 frames) and the zoom it ends at.
const FADE: f32 = 0.5;
const ZOOM: f32 = 1.5;

/// Render pipelines still compiling (the render world's count, a frame or two behind; compute ones aren't counted).
#[derive(Resource, Clone, Default)]
pub(crate) struct Compiling(Arc<AtomicUsize>);

impl Compiling {
    /// Whether any render pipeline is still compiling.
    pub(crate) fn busy(&self) -> bool {
        self.0.load(Ordering::Relaxed) > 0
    }
}

/// P31: the render side's last frame didn't settle yet: it uploaded new textures or meshes (a material whose textures
/// land late isn't specialised, so its pipelines aren't queued yet: `Compiling` can read 0 before they are) or took
/// longer than [`SETTLED`] (an upload, a compile holding a lock, the GPU catching up). Without a renderer it never is.
#[derive(Resource, Clone, Default)]
struct Unsettled(Arc<AtomicBool>);

/// The longest render frame (from the last one's end) that counts as settled.
const SETTLED: std::time::Duration = std::time::Duration::from_millis(100);

/// Whether the render frame that just ended uploaded something or took long (`Unsettled`), since `last` ended.
fn unsettled(
    images: Res<bevy::render::render_asset::ExtractedAssets<bevy::render::texture::GpuImage>>,
    meshes: Res<bevy::render::render_asset::ExtractedAssets<bevy::render::mesh::RenderMesh>>,
    u: Res<Unsettled>,
    mut last: Local<Option<bevy::platform::time::Instant>>,
) {
    let now = bevy::platform::time::Instant::now();
    let slow = last.is_some_and(|t| now.saturating_duration_since(t) > SETTLED);
    *last = Some(now);
    u.0.store(slow || !images.added.is_empty() || !meshes.added.is_empty(), Ordering::Relaxed);
}

pub fn plugin(app: &mut App) {
    let compiling = Compiling::default();
    let settle = Unsettled::default();
    if let Some(r) = app.get_sub_app_mut(bevy::render::RenderApp) {
        r.insert_resource(settle.clone()).add_systems(bevy::render::Render, unsettled.in_set(bevy::render::RenderSystems::Cleanup));
        r.insert_resource(compiling.clone()).add_systems(
            bevy::render::Render,
            (|cache: Res<bevy::render::render_resource::PipelineCache>, c: Res<Compiling>| {
                // render pipelines only: on WebGL2 (no compute) Bevy's sparse buffer update compute pipeline stays
                // queued for good, which held every loading screen to its cap (B101)
                let render = |id| matches!(cache.pipelines().nth(id).map(|p| &p.descriptor), Some(bevy::render::render_resource::PipelineDescriptor::RenderPipelineDescriptor(_)));
                c.0.store(cache.waiting_pipelines().filter(|&id| render(id)).count(), Ordering::Relaxed)
            })
                .in_set(bevy::render::RenderSystems::Cleanup),
        );
    }
    app.init_resource::<Next>()
        .insert_resource(compiling)
        .insert_resource(settle)
        .add_systems(First, (switch, |w: &mut World| run(w, First), loaded).chain())
        .add_systems(Update, zoom)
        .add_systems(First, unwarm.before(switch))
        .add_systems(PostUpdate, warm.before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate))
        .add_systems(PreUpdate, hush.after(bevy::input::InputSystems))
        .add_systems(PreUpdate, (|w: &mut World| run(w, PreUpdate)).after(hush))
        // before the fixed ticks (the court intro holds them: `play/intro.rs`)
        .add_systems(RunFixedMainLoop, (|w: &mut World| run(w, RunFixedMainLoop)).in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop))
        .add_systems(FixedPreUpdate, |w: &mut World| run(w, FixedPreUpdate))
        .add_systems(FixedUpdate, |w: &mut World| {
            if w.get_resource::<Gate>().map(|g| g.0).is_none_or(|gate| gate(w)) {
                run(w, FixedUpdate)
            }
        })
        .add_systems(FixedPostUpdate, |w: &mut World| run(w, FixedPostUpdate))
        // before the base's `texture_faces` (after the match's `animate`) and the UI layout (after `widescreen`'s fit);
        // after the base's `orbit`, or its rig transform could land over the intro's and the ceremony's camera (B288)
        .add_systems(Update, (|w: &mut World| run(w, Update)).before(crate::character::texture_faces).after(crate::orbit))
        .add_systems(PostUpdate, (|w: &mut World| run(w, PostUpdate)).before(bevy::ui::UiSystems::Layout).before(warm))
        // before winit drops the windows on an `AppExit`
        .add_systems(Last, ((|w: &mut World| run(w, Last)), back).chain().before(bevy::window::ExitSystems));
}

/// B200: the running mode holds every press back too (an online match until every peer has loaded it).
#[derive(Resource)]
pub struct Hush;

/// B200: whether this mode's loading screen is still up.
pub fn covered(world: &mut World) -> bool {
    world.query_filtered::<(), With<Loading>>().iter(world).next().is_some()
}

/// B200: no press reaches anything while a switch is pending (the old mode runs on behind the coming loading screen),
/// the loading screen is up, or the mode asks ([`Hush`]): this frame's presses and releases are dropped, held buttons
/// stay held, so one held through counts again only once released and pressed anew.
pub(crate) fn hush(
    next: Res<Next>,
    loading: Query<(), With<Loading>>,
    hushed: Option<Res<Hush>>,
    keys: Option<ResMut<ButtonInput<KeyCode>>>,
    mouse: Option<ResMut<ButtonInput<MouseButton>>>,
    touches: Option<ResMut<Touches>>,
    mut pads: Query<&mut Gamepad>,
) {
    if next.0.is_none() && loading.is_empty() && hushed.is_none() {
        return;
    }
    if let Some(mut k) = keys {
        k.clear();
    }
    if let Some(mut m) = mouse {
        m.clear();
    }
    if let Some(mut t) = touches {
        t.clear();
    }
    for mut g in &mut pads {
        g.digital_mut().clear();
    }
}

/// The running mode's fixed tick (rollback's gate runs the ticks itself).
pub fn run_fixed(world: &mut World) {
    run(world, FixedUpdate)
}

/// The running mode's schedule `label`, if it has one.
fn run(world: &mut World, label: impl ScheduleLabel + Clone) {
    let Some(mut s) = world.get_resource_mut::<Running>().and_then(|mut r| r.s.remove(label)) else { return };
    s.run(world);
    if let Some(mut r) = world.get_resource_mut::<Running>() {
        r.s.insert(s);
    }
}

fn switch(world: &mut World) {
    if world.resource::<Next>().0.is_none() {
        return;
    }
    // the loading screen goes up and is presented (pipelined rendering: frames later) before the build blocks
    if leave(world) {
        return;
    }
    let fading = world.query::<&Snapshot>().iter(world).next().is_some();
    let mut q = world.query::<&mut Loading>();
    match q.iter_mut(world).next() {
        None => {
            show_loading(world);
            return;
        }
        Some(mut l) if l.up < 3 || fading => {
            l.up += 1;
            return;
        }
        Some(mut l) => *l = Loading { up: l.up, built: true, ..default() },
    }
    let next = world.resource_mut::<Next>().0.take().expect("checked");
    if !world.contains_resource::<Base>() {
        let entities = world.iter_entities().map(|e| e.id()).collect();
        let resources = world.iter_resources().map(|(i, _)| i.id()).collect();
        let clear = world.get_resource::<ClearColor>().cloned().unwrap_or_default();
        world.insert_resource(Base { entities, resources, clear });
        let id = world.component_id::<Base>().expect("just inserted");
        world.resource_mut::<Base>().resources.insert(id);
    }
    teardown(world);
    // the plugins build into the real world with empty schedules, as Bevy's own `SubApp::run_as_app`
    let schedules = world.remove_resource::<Schedules>().expect("schedules");
    world.insert_resource(Schedules::new());
    let mut app = App::empty();
    std::mem::swap(app.world_mut(), world);
    (next.build)(&mut app);
    std::mem::swap(app.world_mut(), world);
    let mut s = world.remove_resource::<Schedules>().expect("the mode's schedules");
    world.insert_resource(schedules);
    for label in [PreStartup.intern(), Startup.intern(), PostStartup.intern()] {
        if let Some(mut x) = s.remove(label) {
            x.run(world);
        }
    }
    world.insert_resource(Running { s, back: next.back, leaving: false });
}

/// Before the loading screen: ask for the running mode's last frame and wait for it (10 frames at most: no window,
/// no renderer). True while waiting.
fn leave(world: &mut World) -> bool {
    if world.query::<&Loading>().iter(world).next().is_some() || world.query::<&Snapshot>().iter(world).next().is_some() {
        for e in world.query_filtered::<Entity, With<Leaving>>().iter(world).collect::<Vec<_>>() {
            world.despawn(e);
        }
        return false;
    }
    let can = world.contains_resource::<Running>() && world.query_filtered::<(), With<PrimaryWindow>>().iter(world).next().is_some();
    let mut q = world.query::<&mut Leaving>();
    match q.iter_mut(world).next() {
        Some(mut l) if l.waited < 10 => {
            l.waited += 1;
            true
        }
        Some(_) => false,
        None if can => {
            use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
            world.spawn(Leaving { waited: 0 });
            world.spawn(Screenshot::primary_window()).observe(|ev: On<ScreenshotCaptured>, images: Option<ResMut<Assets<Image>>>, mut c: Commands| {
                let Some(mut images) = images else { return };
                // the capture comes back CPU-only: upload it to draw it
                let mut image = ev.image.clone();
                image.asset_usage = bevy::asset::RenderAssetUsages::default();
                let node = Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
                c.spawn((Snapshot { t: 0.0 }, node, ImageNode::new(images.add(image)), UiTransform::default(), GlobalZIndex(i32::MAX)));
            });
            true
        }
        None => false,
    }
}

/// The last frame zooms up about the centre and fades out (real time: game time is paused under the loading screen).
fn zoom(time: Res<Time<Real>>, mut q: Query<(Entity, &mut Snapshot, &mut UiTransform, &mut ImageNode)>, mut c: Commands) {
    for (e, mut s, mut t, mut image) in &mut q {
        s.t += time.delta_secs();
        let k = (s.t / FADE).min(1.0);
        t.scale = Vec2::splat(1.0 + (ZOOM - 1.0) * k);
        image.color = Color::WHITE.with_alpha(1.0 - k);
        if k >= 1.0 {
            c.entity(e).despawn();
        }
    }
}

fn show_loading(world: &mut World) {
    if let Some(mut t) = world.get_resource_mut::<Time<Virtual>>() {
        t.pause();
    }
    // a plain black cover while the new mode loads (the old "Loading..." text was a placeholder: user 2026-10-11)
    let node = Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() };
    let e = world.spawn((Loading::default(), node, BackgroundColor(Color::BLACK), GlobalZIndex(i32::MAX - 1))).id();
    if let Some(cover) = world.resource_mut::<Next>().0.as_mut().and_then(|m| m.cover.take()) {
        cover(world, e);
    }
}

/// Takes the loading screen down once the new mode has drawn a few frames with no pipeline left compiling and nothing
/// left uploading (10 s at most), and starts game time again.
fn loaded(world: &mut World) {
    let compiling = world.get_resource::<Compiling>().map_or(0, |c| c.0.load(Ordering::Relaxed));
    let unsettled = world.get_resource::<Unsettled>().is_some_and(|u| u.0.load(Ordering::Relaxed));
    let mut q = world.query::<(Entity, &mut Loading)>();
    let Some((e, mut l)) = q.iter_mut(world).next().filter(|(_, l)| l.built) else { return };
    l.after += 1;
    l.idle = if compiling == 0 && !unsettled { l.idle + 1 } else { 0 };
    if (l.after < 5 || l.idle < 3) && l.after < 600 {
        return;
    }
    info!("loading screen down after {} frames at {:.2} s", l.after, crate::stall::since_start());
    crate::stall::watch();
    world.entity_mut(e).despawn();
    if let Some(mut t) = world.get_resource_mut::<Time<Virtual>>() {
        t.unpause();
    }
}

/// A mesh drawn under the loading screen this frame: its own visibility and whether it was frustum culled.
#[derive(Component)]
struct Warming(Visibility, bool);

/// While the loading screen is up over the new mode, every mesh is drawn under it, hidden or out of view too, so the
/// pipelines a hidden effect needs (a hit's sparks, a lob's landing marks) compile behind the cover, not on its first
/// show mid-match: the browser compiles them on the main thread (B110). `loaded` waits for them.
fn warm(
    mut commands: Commands,
    loading: Query<&Loading>,
    mut q: Query<(Entity, &mut Visibility, Has<bevy::camera::visibility::NoFrustumCulling>), (With<Mesh3d>, Without<Warming>)>,
    mut cams: Query<(Entity, &mut Camera), Without<WarmingCamera>>,
) {
    if !loading.iter().any(|l| l.built) {
        return;
    }
    // an overlay camera off until its moment (the umpire's call models, B155) draws too: its view's pipelines
    for (e, mut c) in &mut cams {
        if !c.is_active {
            c.is_active = true;
            commands.entity(e).insert(WarmingCamera);
        }
    }
    for (e, mut v, unculled) in &mut q {
        commands.entity(e).insert((Warming(*v, !unculled), bevy::camera::visibility::NoFrustumCulling));
        *v = Visibility::Visible;
    }
}

/// A camera made active under the loading screen (it was off).
#[derive(Component)]
struct WarmingCamera;

/// Each warmed mesh and camera back as its systems left it, before they run again.
fn unwarm(mut commands: Commands, mut q: Query<(Entity, &mut Visibility, &Warming)>, mut cams: Query<(Entity, &mut Camera), With<WarmingCamera>>) {
    for (e, mut c) in &mut cams {
        c.is_active = false;
        commands.entity(e).remove::<WarmingCamera>();
    }
    for (e, mut v, w) in &mut q {
        *v = w.0;
        let mut e = commands.entity(e);
        e.remove::<Warming>();
        if w.1 {
            e.remove::<bevy::camera::visibility::NoFrustumCulling>();
        }
    }
}

/// Drop the running mode: its entities (except windows, monitors and pads) and the resources of this crate it added.
fn teardown(world: &mut World) {
    world.remove_resource::<Running>();
    if let Some(s) = world.get_resource::<crate::audio::Sound>() {
        s.stop_music();
        s.end_match();
    }
    let base = world.remove_resource::<Base>().expect("base");
    let doomed: Vec<Entity> = world
        .iter_entities()
        .filter(|e| !base.entities.contains(&e.id()) && e.get::<ChildOf>().is_none_or(|p| base.entities.contains(&p.parent())))
        .filter(|e| !(e.contains::<Loading>() || e.contains::<IsResource>() || e.contains::<Window>() || e.contains::<Monitor>() || e.contains::<Gamepad>() || e.contains::<Observer>()))
        .map(|e| e.id())
        .collect();
    for e in doomed {
        if let Ok(e) = world.get_entity_mut(e) {
            e.despawn();
        }
    }
    // this crate's only: Bevy inserts some of its own as they're first used (`AssetChanges`); names need Bevy's `debug`
    let ours: Vec<ComponentId> = world.iter_resources().filter(|(i, _)| !base.resources.contains(&i.id()) && i.name().to_string().starts_with("hst::")).map(|(i, _)| i.id()).collect();
    for id in ours {
        world.remove_resource_by_id(id);
    }
    // base resources a mode changes: the clear colour (weather), the fixed clock (the pause menu holds it)
    world.insert_resource(base.clear.clone());
    world.insert_resource(Time::<Fixed>::from_hz(60.0));
    world.insert_resource(base);
}

/// The running mode's `AppExit` → its way back, while the primary window is open (closing it still quits).
fn back(world: &mut World) {
    if world.get_resource::<Running>().is_none_or(|r| r.back.is_none() && !r.leaving) || world.resource::<Messages<AppExit>>().is_empty() || world.contains_resource::<Quit>() {
        return;
    }
    if world.query_filtered::<(), With<PrimaryWindow>>().iter(world).next().is_none() {
        return;
    }
    world.resource_mut::<Messages<AppExit>>().clear();
    // the mode runs on for a few frames while it leaves (`leave`) and may ask again (the pause menu's quit does every
    // frame once its fade is done): that's the same way back, not a quit
    let Some(to) = world.resource_mut::<Running>().back.take() else { return };
    world.resource_mut::<Next>().0 = Some(*to);
    world.resource_mut::<Running>().leaving = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct Count(u32);
    #[derive(Component)]
    struct Thing;
    /// The test's quit: mode 2 asks for its way back every frame while this is in (as the pause menu does).
    #[derive(Resource)]
    struct Quit;

    /// B200: a press while a switch is pending (or the loading screen is up, or the mode asks) reaches nothing, and a
    /// button held through it doesn't count as pressed afterwards.
    #[test]
    fn presses_wait_out_a_switch() {
        let mut app = App::new();
        app.init_resource::<Next>().init_resource::<ButtonInput<KeyCode>>().add_systems(Update, hush);
        let mut press = |app: &mut App, k| {
            app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(k);
            app.update();
            let keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            (keys.just_pressed(k), keys.pressed(k))
        };
        assert_eq!(press(&mut app, KeyCode::Enter), (true, true));
        app.world_mut().resource_mut::<Next>().0 = Some(Mode::new(|_| {}));
        assert_eq!(press(&mut app, KeyCode::Space), (false, true));
        app.world_mut().resource_mut::<Next>().0 = None;
        app.insert_resource(Hush);
        assert_eq!(press(&mut app, KeyCode::KeyJ), (false, true));
        app.world_mut().remove_resource::<Hush>();
        // still held: no new press (a key repeat presses it again)
        assert_eq!(press(&mut app, KeyCode::KeyJ), (false, true));
    }

    fn mode(tag: u32) -> Mode {
        Mode::new(move |app| {
            app.init_resource::<Count>()
                .add_systems(Startup, move |mut c: Commands| _ = c.spawn(Thing))
                .add_systems(Update, move |mut n: ResMut<Count>, quit: Option<Res<Quit>>, mut exit: MessageWriter<AppExit>| {
                    n.0 = n.0.max(tag * 100) + 1;
                    if tag == 2 && quit.is_some() {
                        exit.write(AppExit::Success);
                    }
                });
        })
    }

    /// Frames until the switch (the loading screen's and the old mode's last frame, if they aren't done yet), the
    /// switch's included; nothing quits meanwhile.
    fn loading(app: &mut App) {
        while app.world().resource::<Next>().0.is_some() {
            app.update();
            assert!(app.should_exit().is_none(), "a mode asking again while it leaves still goes back");
        }
    }

    /// Frames until the loading screen is down.
    fn settle(app: &mut App) {
        while app.world_mut().query::<&Loading>().iter(app.world()).next().is_some() {
            app.update();
        }
    }

    fn count(app: &App) -> u32 {
        app.world().resource::<Count>().0
    }

    /// Menu → match → (the match's quit) → menu: each start builds fresh (`Local`s, resources, Startup), the old
    /// mode's entities go, the base's stay, and only a mode without a way back quits.
    #[test]
    fn switches_and_comes_back() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::window::WindowPlugin { primary_window: Some(Window::default()), exit_condition: bevy::window::ExitCondition::DontExit, ..default() }, plugin));
        let kept = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<Next>().0 = Some(mode(1));
        loading(&mut app);
        settle(&mut app);
        assert!((101..200).contains(&count(&app)));
        app.world_mut().resource_mut::<Next>().0 = Some(mode(2).back(mode(1)));
        loading(&mut app);
        assert!((201..300).contains(&count(&app)), "fresh resource");
        assert_eq!(app.world_mut().query::<&Thing>().iter(app.world()).count(), 1, "the menu's Thing went");
        settle(&mut app);
        app.insert_resource(Quit);
        app.update();
        assert!(app.should_exit().is_none(), "the match's quit goes back");
        loading(&mut app);
        assert!((101..200).contains(&count(&app)), "back in a fresh menu");
        assert_eq!(app.world_mut().query::<&Thing>().iter(app.world()).count(), 1);
        assert!(app.world().get_entity(kept).is_ok());
        settle(&mut app);
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
        assert!(app.world_mut().query::<&Window>().iter(app.world()).next().is_some());
        app.world_mut().write_message(AppExit::Success);
        app.update();
        assert!(app.should_exit().is_some(), "the menu's quit quits");
    }

    /// B111: leaving a match (here its quit while paused) cuts every sound it started, the paused ones included;
    /// the menu's sounds and the pause screen's own aren't the match's, and the next mode's aren't tagged.
    #[test]
    fn match_sounds_end_with_the_match() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::window::WindowPlugin { primary_window: Some(Window::default()), exit_condition: bevy::window::ExitCondition::DontExit, ..default() }, plugin));
        let sound = crate::audio::Sound::bare();
        app.insert_resource(sound.clone());
        let menu = sound.endless();
        app.world_mut().resource_mut::<Next>().0 = Some(mode(1));
        loading(&mut app);
        settle(&mut app);
        let s = sound.clone();
        app.world_mut().resource_mut::<Next>().0 = Some(mode(2).and(move |_| s.begin_match()).back(mode(1)));
        loading(&mut app);
        settle(&mut app);
        let (rain, lob) = (sound.endless(), sound.endless());
        sound.hold(true);
        let blip = sound.endless();
        assert_eq!(sound.match_sounds(), 2);
        app.insert_resource(Quit);
        app.update();
        loading(&mut app);
        assert_eq!(sound.match_sounds(), 0);
        assert!(!sound.playing(rain) && !sound.playing(lob));
        assert!(sound.playing(menu) && sound.playing(blip));
        sound.endless();
        assert_eq!(sound.match_sounds(), 0, "the menu's sounds aren't tagged");
    }

    /// An unattended run's quit (`Quit` in with the `AppExit`: `--shot` done) quits even a mode with a way back.
    #[test]
    fn a_real_quit_quits_a_match() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::window::WindowPlugin { primary_window: Some(Window::default()), exit_condition: bevy::window::ExitCondition::DontExit, ..default() }, plugin));
        app.world_mut().resource_mut::<Next>().0 = Some(mode(2).back(mode(1)));
        loading(&mut app);
        settle(&mut app);
        app.insert_resource(super::Quit);
        app.world_mut().write_message(AppExit::Success);
        app.update();
        assert!(app.should_exit().is_some());
    }
}
