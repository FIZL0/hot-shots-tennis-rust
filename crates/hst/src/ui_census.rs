//! `HST_UI_CENSUS=1` (P39): every 5 s, to stderr, how many UI nodes each UI root holds and how many of them were
//! changed per frame (Node, Text, ComputedNode, UiGlobalTransform, …), with the root's components to tell which tree it
//! is. Bevy's UI systems (layout, stack, clipping, focus) walk every node every frame, so node count and per-frame
//! changes are what they cost.

use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use std::collections::BTreeMap;

pub fn plugin(app: &mut App) {
    app.init_resource::<Census>().add_systems(Last, (count, report).chain());
}

#[derive(Resource, Default)]
struct Census {
    frames: u32,
    /// per root: [Node, Text, TextFont/Color, ComputedNode, UiGlobalTransform, BackgroundColor/ImageNode, Visibility]
    changed: BTreeMap<Entity, [u32; 7]>,
    /// a changed node per root, to name it
    sample: BTreeMap<Entity, Entity>,
    since: f32,
}

fn root_of(mut e: Entity, parents: &Query<&ChildOf>) -> Entity {
    while let Ok(p) = parents.get(e) {
        e = p.parent();
    }
    e
}

#[allow(clippy::type_complexity)]
fn count(
    mut c: ResMut<Census>,
    parents: Query<&ChildOf>,
    q: Query<(Entity, Ref<Node>, Option<Ref<Text>>, Option<Ref<TextColor>>, Option<Ref<TextFont>>, Ref<ComputedNode>, Ref<UiGlobalTransform>, Option<Ref<BackgroundColor>>, Option<Ref<ImageNode>>, Option<Ref<Visibility>>)>,
) {
    c.frames += 1;
    for (e, n, t, tc, tf, cn, gt, bg, im, v) in &q {
        let flags = [
            n.is_changed() && !n.is_added(),
            t.is_some_and(|t| t.is_changed() && !t.is_added()),
            tc.is_some_and(|t| t.is_changed() && !t.is_added()) || tf.is_some_and(|t| t.is_changed() && !t.is_added()),
            cn.is_changed() && !cn.is_added(),
            gt.is_changed() && !gt.is_added(),
            bg.is_some_and(|t| t.is_changed() && !t.is_added()) || im.is_some_and(|t| t.is_changed() && !t.is_added()),
            v.is_some_and(|t| t.is_changed() && !t.is_added()),
        ];
        if flags.iter().any(|&f| f) {
            let r = root_of(e, &parents);
            c.sample.insert(r, e);
            let row = c.changed.entry(r).or_default();
            for (i, f) in flags.iter().enumerate() {
                row[i] += *f as u32;
            }
        }
    }
}

fn report(world: &mut World) {
    let dt = world.resource::<Time<Real>>().delta_secs();
    let mut c = world.resource_mut::<Census>();
    c.since += dt;
    if c.since < 5.0 {
        return;
    }
    let Census { frames, changed, sample, .. } = std::mem::take(&mut *c);
    let mut sizes: BTreeMap<Entity, u32> = BTreeMap::new();
    let mut q = world.query_filtered::<Entity, With<Node>>();
    let nodes: Vec<Entity> = q.iter(world).collect();
    for e in &nodes {
        let mut r = *e;
        while let Some(p) = world.get::<ChildOf>(r) {
            r = p.parent();
        }
        *sizes.entry(r).or_default() += 1;
    }
    eprintln!("ui census: {} nodes in {} roots, {frames} frames; per frame changed [node text font computed gtransform bg/img vis]", nodes.len(), sizes.len());
    let mut roots: Vec<_> = sizes.into_iter().collect();
    roots.sort_by_key(|r| std::cmp::Reverse(r.1));
    for (r, n) in roots {
        let ch = changed.get(&r).copied().unwrap_or_default().map(|v| (v as f32 / frames.max(1) as f32 * 10.0).round() / 10.0);
        let comps: Vec<String> = world
            .inspect_entity(r)
            .map(|it| it.map(|i| i.name().shortname().to_string()).filter(|n| !n.starts_with("Computed") && !n.contains("Transform") && !n.contains("Visibility")).collect())
            .unwrap_or_default();
        let display = world.get::<Node>(r).map(|n| n.display);
        let vis = world.get::<Visibility>(r).copied();
        eprintln!("  {n:4} {r} {display:?} {vis:?} {ch:?} {}", comps.join(","));
        if let Some(&e) = sample.get(&r).filter(|&&e| e != r) {
            let comps: Vec<String> = world.inspect_entity(e).map(|it| it.map(|i| i.name().shortname().to_string()).filter(|n| !n.starts_with("Computed") && !n.contains("Transform") && !n.contains("Visibility") && !n.starts_with("Inherited") && !n.starts_with("Propagate")).collect()).unwrap_or_default();
            eprintln!("       changed e.g. {e}: {}", comps.join(","));
        }
    }
}
