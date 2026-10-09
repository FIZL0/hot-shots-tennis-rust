//! Modes of one app: the main menu and a match. A mode is a set of plugins built into fresh schedules when it starts
//! (fresh resources and `Local`s, its Startup run once) and torn down when the next one starts: its entities and this
//! crate's resources it added go. The window, the pads, the audio device, the GPU and loaded textures stay up. A mode
//! builds without the render app, so Bevy plugins with a render side (materials) go in the base app.
//! A mode with a way back (the match → the menu) turns its `AppExit` into that switch while the window is open.

use bevy::ecs::component::ComponentId;
use bevy::ecs::message::Messages;
use bevy::ecs::resource::IsResource;
use bevy::ecs::schedule::{ScheduleLabel, Schedules};
use bevy::input::gamepad::Gamepad;
use bevy::prelude::*;
use bevy::window::{Monitor, PrimaryWindow};
use std::collections::HashSet;

type Build = Box<dyn FnOnce(&mut App) + Send + Sync>;

pub struct Mode {
    build: Build,
    back: Option<Box<Mode>>,
}

impl Mode {
    pub fn new(build: impl FnOnce(&mut App) + Send + Sync + 'static) -> Self {
        Mode { build: Box::new(build), back: None }
    }

    /// Where this mode's `AppExit` goes instead of closing the app.
    pub fn back(mut self, to: Mode) -> Self {
        self.back = Some(Box::new(to));
        self
    }
}

/// The mode to switch to at the start of the next frame.
#[derive(Resource, Default)]
pub struct Next(pub Option<Mode>);

/// The running mode's schedules.
#[derive(Resource)]
struct Running {
    s: Schedules,
    back: Option<Box<Mode>>,
}

/// What the app had before the first mode: kept through every switch.
#[derive(Resource)]
struct Base {
    entities: HashSet<Entity>,
    resources: HashSet<ComponentId>,
    clear: ClearColor,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Next>()
        .add_systems(First, (switch, |w: &mut World| run(w, First)).chain())
        .add_systems(PreUpdate, |w: &mut World| run(w, PreUpdate))
        .add_systems(FixedPreUpdate, |w: &mut World| run(w, FixedPreUpdate))
        .add_systems(FixedUpdate, |w: &mut World| run(w, FixedUpdate))
        .add_systems(FixedPostUpdate, |w: &mut World| run(w, FixedPostUpdate))
        // before the base's `texture_faces` (after the match's `animate`) and the UI layout (after `widescreen`'s fit)
        .add_systems(Update, (|w: &mut World| run(w, Update)).before(crate::character::texture_faces))
        .add_systems(PostUpdate, (|w: &mut World| run(w, PostUpdate)).before(bevy::ui::UiSystems::Layout))
        // before winit drops the windows on an `AppExit`
        .add_systems(Last, ((|w: &mut World| run(w, Last)), back).chain().before(bevy::window::ExitSystems));
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
    let Some(next) = world.resource_mut::<Next>().0.take() else { return };
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
    world.insert_resource(Running { s, back: next.back });
}

/// Drop the running mode: its entities (except windows, monitors and pads) and the resources of this crate it added.
fn teardown(world: &mut World) {
    world.remove_resource::<Running>();
    let base = world.remove_resource::<Base>().expect("base");
    let doomed: Vec<Entity> = world
        .iter_entities()
        .filter(|e| !base.entities.contains(&e.id()) && e.get::<ChildOf>().is_none_or(|p| base.entities.contains(&p.parent())))
        .filter(|e| !(e.contains::<IsResource>() || e.contains::<Window>() || e.contains::<Monitor>() || e.contains::<Gamepad>() || e.contains::<Observer>()))
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
    if world.get_resource::<Running>().is_none_or(|r| r.back.is_none()) || world.resource::<Messages<AppExit>>().is_empty() {
        return;
    }
    if world.query_filtered::<(), With<PrimaryWindow>>().iter(world).next().is_none() {
        return;
    }
    world.resource_mut::<Messages<AppExit>>().clear();
    let to = world.resource_mut::<Running>().back.take();
    world.resource_mut::<Next>().0 = to.map(|b| *b);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct Count(u32);
    #[derive(Component)]
    struct Thing;

    fn mode(tag: u32) -> Mode {
        Mode::new(move |app| {
            app.init_resource::<Count>()
                .add_systems(Startup, move |mut c: Commands| _ = c.spawn(Thing))
                .add_systems(Update, move |mut n: ResMut<Count>, mut ticks: Local<u32>, mut exit: MessageWriter<AppExit>| {
                    *ticks += 1;
                    n.0 = n.0.max(tag * 100) + 1;
                    if tag == 2 && *ticks == 3 {
                        exit.write(AppExit::Success);
                    }
                });
        })
    }

    /// Menu → match → (the match's quit) → menu: each start builds fresh (`Local`s, resources, Startup), the old
    /// mode's entities go, the base's stay, and only a mode without a way back quits.
    #[test]
    fn switches_and_comes_back() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::window::WindowPlugin { primary_window: Some(Window::default()), exit_condition: bevy::window::ExitCondition::DontExit, ..default() }, plugin));
        let kept = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<Next>().0 = Some(mode(1));
        app.update();
        app.update();
        assert_eq!(app.world().resource::<Count>().0, 102);
        app.world_mut().resource_mut::<Next>().0 = Some(mode(2).back(mode(1)));
        app.update();
        assert_eq!(app.world().resource::<Count>().0, 201, "fresh resource and Local");
        assert_eq!(app.world_mut().query::<&Thing>().iter(app.world()).count(), 1, "the menu's Thing went");
        app.update();
        app.update();
        assert!(app.should_exit().is_none(), "the match's quit goes back");
        app.update();
        assert_eq!(app.world().resource::<Count>().0, 101, "back in a fresh menu");
        assert_eq!(app.world_mut().query::<&Thing>().iter(app.world()).count(), 1);
        assert!(app.world().get_entity(kept).is_ok());
        assert!(app.world_mut().query::<&Window>().iter(app.world()).next().is_some());
        app.world_mut().write_message(AppExit::Success);
        app.update();
        assert!(app.should_exit().is_some(), "the menu's quit quits");
    }
}
