//! `HST_SCENE_CENSUS=1` (P41): every 5 s, to stderr, what Bevy's visibility and transform systems walk each frame:
//! entities with a `ViewVisibility` (every active camera's CPU culling visits each of them), meshes per root (and how
//! many are hidden), active cameras with a frustum, and how many `GlobalTransform`s changed per frame.

use bevy::camera::primitives::Frustum;
use bevy::prelude::*;
use std::collections::BTreeMap;

pub fn plugin(app: &mut App) {
    app.init_resource::<Census>().add_systems(Last, (count, report).chain());
}

#[derive(Resource, Default)]
struct Census {
    frames: u32,
    moved: u32,
    /// moved transforms per component set
    kinds: BTreeMap<String, u32>,
    since: f32,
}

fn count(world: &mut World) {
    let mut q = world.query::<(Entity, Ref<GlobalTransform>)>();
    let moved: Vec<Entity> = q.iter(world).filter(|(_, t)| t.is_changed() && !t.is_added()).map(|(e, _)| e).collect();
    let kinds: Vec<String> = moved.iter().map(|e| set(world, *e)).collect();
    let mut c = world.resource_mut::<Census>();
    c.frames += 1;
    c.moved += moved.len() as u32;
    for k in kinds {
        *c.kinds.entry(k).or_default() += 1;
    }
}

/// An entity's components, less the transform/visibility/hierarchy ones every entity here has; and its root's
fn set(world: &World, e: Entity) -> String {
    let names = |e: Entity| -> Vec<String> {
        world
            .inspect_entity(e)
            .map(|it| it.map(|i| i.name().shortname().to_string()).filter(|n| !n.contains("Transform") && !n.contains("Visibility") && n != "ChildOf" && n != "Children" && !n.starts_with("Inherited")).collect())
            .unwrap_or_default()
    };
    let mut r = e;
    while let Some(p) = world.get::<ChildOf>(r) {
        r = p.parent();
    }
    let root: Vec<String> = names(r).into_iter().filter(|n| !n.starts_with("Camera") && !n.starts_with("Render") && !n.starts_with("Visible")).take(4).collect();
    format!("{} | root {}", names(e).join(","), root.join(","))
}

fn report(world: &mut World) {
    let dt = world.resource::<Time<Real>>().delta_secs();
    let mut c = world.resource_mut::<Census>();
    c.since += dt;
    if c.since < 5.0 {
        return;
    }
    let Census { frames, moved, kinds, .. } = std::mem::take(&mut *c);
    let all = world.entities().count_spawned();
    let mut q = world.query::<(Entity, &InheritedVisibility, &ViewVisibility, Has<Mesh3d>, Has<Node>)>();
    let rows: Vec<_> = q.iter(world).map(|(e, i, v, m, n)| (e, i.get(), v.get(), m, n)).collect();
    let (vis, shown, seen) = (rows.len(), rows.iter().filter(|r| r.1).count(), rows.iter().filter(|r| r.2).count());
    let ui = rows.iter().filter(|r| r.4).count();
    let mut gt = world.query::<&GlobalTransform>();
    let gts = gt.iter(world).count();
    eprintln!(
        "scene census: {all} entities, {gts} transforms ({:.1} moved/frame), {vis} with ViewVisibility ({ui} UI), {shown} inherited-visible, {seen} seen",
        moved as f32 / frames.max(1) as f32
    );
    let mut kinds: Vec<_> = kinds.into_iter().collect();
    kinds.sort_by_key(|k| std::cmp::Reverse(k.1));
    for (k, n) in kinds.iter().take(10) {
        eprintln!("    moved {:6.1}/frame {k}", *n as f32 / frames.max(1) as f32);
    }
    let mut cams = world.query::<(Entity, &Camera, Has<Frustum>, Has<Camera3d>, Has<Camera2d>)>();
    let cams: Vec<_> = cams.iter(world).map(|(e, c, f, d3, d2)| (e, c.is_active, c.order, f, d3, d2)).collect();
    for (e, active, order, f, d3, d2) in cams {
        eprintln!("  camera {e} active {active} order {order} frustum {f} 3d {d3} 2d {d2}");
    }
    // meshes per root: [total, inherited-visible]
    let mut roots: BTreeMap<Entity, [u32; 2]> = BTreeMap::new();
    for (e, shown, ..) in rows.iter().filter(|r| r.3) {
        let mut r = *e;
        while let Some(p) = world.get::<ChildOf>(r) {
            r = p.parent();
        }
        let row = roots.entry(r).or_default();
        row[0] += 1;
        row[1] += *shown as u32;
    }
    let mut roots: Vec<_> = roots.into_iter().collect();
    roots.sort_by_key(|r| std::cmp::Reverse(r.1[0]));
    eprintln!("  meshes {} in {} roots [total, shown]:", roots.iter().map(|r| r.1[0]).sum::<u32>(), roots.len());
    for (r, n) in roots.iter().take(12) {
        let comps: Vec<String> = world
            .inspect_entity(*r)
            .map(|it| it.map(|i| i.name().shortname().to_string()).filter(|n| !n.contains("Transform") && !n.contains("Visibility")).collect())
            .unwrap_or_default();
        eprintln!("    {n:?} {r} {}", comps.join(","));
    }
    // what else carries a ViewVisibility: the non-mesh, non-UI rest by component set
    let mut other: BTreeMap<String, u32> = BTreeMap::new();
    for (e, ..) in rows.iter().filter(|r| !r.3 && !r.4) {
        *other.entry(set(world, *e)).or_default() += 1;
    }
    // meshes by their parent's kind
    let mut by_parent: BTreeMap<String, [u32; 3]> = BTreeMap::new();
    let mut parents = std::collections::HashSet::new();
    for (e, shown, ..) in rows.iter().filter(|r| r.3) {
        let Some(p) = world.get::<ChildOf>(*e).map(|p| p.parent()) else { continue };
        let row = by_parent.entry(set(world, p)).or_default();
        row[0] += 1;
        row[1] += *shown as u32;
        row[2] += parents.insert(p) as u32;
    }
    let mut by_parent: Vec<_> = by_parent.into_iter().collect();
    by_parent.sort_by_key(|o| std::cmp::Reverse(o.1[0]));
    for (k, n) in by_parent.iter().take(10) {
        eprintln!("    meshes [total, shown, parents] {n:?} under {k}");
    }
    // hidden ones (the views skip them at once, but every pass still visits them)
    let mut hidden: BTreeMap<String, u32> = BTreeMap::new();
    for (e, ..) in rows.iter().filter(|r| !r.1 && !r.4) {
        *hidden.entry(set(world, *e)).or_default() += 1;
    }
    let mut hidden: Vec<_> = hidden.into_iter().collect();
    hidden.sort_by_key(|o| std::cmp::Reverse(o.1));
    for (k, n) in hidden.iter().take(12) {
        eprintln!("    hidden {n:4} {k}");
    }
    let mut other: Vec<_> = other.into_iter().collect();
    other.sort_by_key(|o| std::cmp::Reverse(o.1));
    for (k, n) in other.iter().take(16) {
        eprintln!("    other {n:4} {k}");
    }
}
