//! B477: on a court mod the layout court's walkers (its gallery) stand where the disc court has room for them, which
//! can be inside the mod's walls, stands or props, or over a drop. Each is drawn instead at the nearest spot within
//! [`FAR`] where a body ([`R`] round, from [`KNEE`] to [`HEAD`] over its feet) touches none of the mod court's
//! triangles and there is ground (`court_mod::solid`), standing on that ground, never further into the playing area
//! than its home; one with no such spot isn't drawn. Drawn only: its moves, cheers and the court generator are the
//! layout's, so replays, rollback and online are unchanged. The umpire sits on the court's own chair (B433); the mod's
//! own people (`court_people`) stand where its author put them; the trigger creatures are hidden (B436).

use std::collections::HashMap;

use bevy::prelude::*;

use crate::mods_court::MatchCourt;

/// Body radius (the walkers' own, `npc::Wander`) and the room kept round it for their dodges.
const R: f32 = 0.3 + 0.1;
/// What's lower than this over the feet is stepped over (kerbs, low hedges, steps).
const KNEE: f32 = 0.3;
/// The body's top over the feet.
const HEAD: f32 = 1.7;
/// How far the mod's ground may lie above or below the layout's for a walker to stand on it.
const STEP: f32 = 0.6;
/// How far the ground round the feet may rise or fall.
const LEVEL: f32 = 0.15;
/// How far from a drop a walker stands at least.
const EDGE: f32 = 0.8;
/// How far a walker is moved at most.
const FAR: f32 = 6.0;
/// Half the doubles court plus its run-off (x, z): the players' room.
const PLAY: Vec2 = Vec2::new(5.485 + 2.0, 11.885 + 4.0);
/// The grid's cell (m).
const CELL: f32 = 1.0;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PostStartup, place.after(super::setup).run_if(resource_exists::<MatchCourt>))
        .add_systems(PostUpdate, draw.before(bevy::transform::TransformSystems::Propagate).run_if(resource_exists::<MatchCourt>));
}

/// A walker drawn this far (game space) from where the match has it.
#[derive(Component)]
struct Moved(Vec3);

/// Each walker on the court mod: moved clear of its geometry, or hidden.
fn place(mut commands: Commands, court: Res<MatchCourt>, args: Res<crate::Args>, npcs: Option<Res<super::Npcs>>) {
    let Some(npcs) = npcs else { return };
    let solid = Solid::new(crate::court_mod::solid(&court.0, args.singles));
    let mut taken: Vec<Vec3> = crate::court_mod::umpire_seat(&court.0, args.singles).map(Vec3::from).into_iter().collect();
    for (e, _, home) in &npcs.walkers {
        let home = Vec3::new(home[0], home[1], home[2]);
        match solid.clear(home, &taken) {
            Some(at) => {
                taken.push(at);
                if at != home {
                    info!("court mod {}: a walker at {home} drawn at {at}", court.0.id);
                    commands.entity(*e).insert(Moved(at - home));
                }
            }
            None => {
                info!("court mod {}: no room for the walker at {home}, not drawn", court.0.id);
                commands.entity(*e).insert(Visibility::Hidden);
            }
        }
    }
}

/// The moved walkers where the match has them (`npc::Wander::world`), shifted.
fn draw(npcs: Option<Res<super::Npcs>>, mut q: Query<(&mut Transform, &Moved)>) {
    let Some(npcs) = npcs else { return };
    for (e, w, _) in &npcs.walkers {
        if let Ok((mut t, m)) = q.get_mut(*e) {
            let p = w.wander.world[3];
            t.translation = Vec3::new(p[0], p[1], p[2]) + m.0;
        }
    }
}

/// The court mod's triangles (game space, Y down) by the grid cells their footprints cover.
struct Solid {
    tris: Vec<[Vec3; 3]>,
    cells: HashMap<(i32, i32), Vec<u32>>,
}

impl Solid {
    fn new(tris: Vec<[Vec3; 3]>) -> Solid {
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (k, t) in tris.iter().enumerate() {
            let (lo, hi) = (t[0].min(t[1]).min(t[2]), t[0].max(t[1]).max(t[2]));
            // only what could hold up or touch a body standing near the layout's ground (a tall wall's top is kept:
            // it isn't a drop, `footing`)
            if lo.y > STEP + KNEE || (hi - lo).xz().max_element() > 200.0 {
                continue;
            }
            for x in cell(lo.x)..=cell(hi.x) {
                for z in cell(lo.z)..=cell(hi.z) {
                    cells.entry((x, z)).or_default().push(k as u32);
                }
            }
        }
        Solid { tris, cells }
    }

    /// The triangles whose cells touch the square `r` round `p`.
    fn near(&self, p: Vec2, r: f32) -> impl Iterator<Item = &[Vec3; 3]> {
        let mut ks: Vec<u32> = (cell(p.x - r)..=cell(p.x + r)).flat_map(|x| (cell(p.y - r)..=cell(p.y + r)).map(move |z| (x, z))).filter_map(|c| self.cells.get(&c)).flatten().copied().collect();
        ks.sort_unstable();
        ks.dedup();
        ks.into_iter().map(|k| &self.tris[k as usize])
    }

    /// The lowest ground under `p` (x, z) within [`STEP`] of height `y` (game space: the greatest y): a bench, a
    /// table or a wall's top over the floor is in the way, not somewhere to stand.
    fn ground(&self, p: Vec2, y: f32) -> Option<f32> {
        self.near(p, 0.0).filter_map(|t| height(t, p)).filter(|h| (h - y).abs() <= STEP).max_by(f32::total_cmp)
    }

    /// Ground as level as the feet's all round within the body (not a ledge's edge), and no drop within [`EDGE`]
    /// (not a wall's top): something there no further down than [`STEP`].
    fn footing(&self, feet: Vec3) -> bool {
        let round = |r: f32, ok: &dyn Fn(f32) -> bool| {
            (0..8).all(|k| {
                let p = feet.xz() + Vec2::from_angle(k as f32 * std::f32::consts::FRAC_PI_4) * r;
                self.near(p, 0.0).filter_map(|t| height(t, p)).any(ok)
            })
        };
        round(R - 0.1, &|h| (h - feet.y).abs() <= LEVEL) && round(EDGE, &|h| h - feet.y <= STEP)
    }

    /// A body standing at `feet` touches a triangle.
    fn blocked(&self, feet: Vec3) -> bool {
        let p = feet.xz();
        self.near(p, R).any(|t| touches(t, p, feet.y - HEAD, feet.y - KNEE, R))
    }

    /// Where a walker whose home is `home` is drawn: its home, or the nearest spot with room at least a body apart
    /// from `taken`, its feet on the mod's ground; `None` when there's none within [`FAR`].
    fn clear(&self, home: Vec3, taken: &[Vec3]) -> Option<Vec3> {
        let depth = |p: Vec2| (PLAY - p.abs()).min_element();
        let deepest = depth(home.xz()).max(0.0);
        let spots = std::iter::once(Vec2::ZERO).chain((1..=(FAR / 0.25) as u32).flat_map(|ring| {
            let r = ring as f32 * 0.25;
            let n = (ring * 8).min(64);
            (0..n).map(move |k| Vec2::from_angle(k as f32 * std::f32::consts::TAU / n as f32) * r)
        }));
        spots
            .map(|d| home.xz() + d)
            .filter(|&p| depth(p) <= deepest)
            .filter(|&p| p == home.xz() || taken.iter().all(|t| t.xz().distance(p) >= 2.0 * R))
            .find_map(|p| self.ground(p, home.y).map(|y| Vec3::new(p.x, y, p.y)).filter(|&f| self.footing(f) && !self.blocked(f)))
    }
}

fn cell(v: f32) -> i32 {
    (v / CELL).floor() as i32
}

/// The triangle's height over `p` (x, z), if `p` lies within its footprint.
fn height(t: &[Vec3; 3], p: Vec2) -> Option<f32> {
    let (a, b, c) = (t[0].xz(), t[1].xz(), t[2].xz());
    let d = (b - a).perp_dot(c - a);
    if d.abs() < 1e-8 {
        return None;
    }
    let (u, v) = ((p - a).perp_dot(c - a) / d, (b - a).perp_dot(p - a) / d);
    (u >= 0.0 && v >= 0.0 && u + v <= 1.0).then(|| t[0].y + u * (t[1].y - t[0].y) + v * (t[2].y - t[0].y))
}

/// The part of the triangle between heights `lo` and `hi` (game space y) comes within `r` of `p` (x, z): an upright
/// cylinder's test, the triangle clipped to its slab and seen from above.
fn touches(t: &[Vec3; 3], p: Vec2, lo: f32, hi: f32, r: f32) -> bool {
    let clip = |poly: Vec<Vec3>, keep: &dyn Fn(f32) -> f32| -> Vec<Vec3> {
        let mut out = Vec::with_capacity(poly.len() + 1);
        for (k, &a) in poly.iter().enumerate() {
            let b = poly[(k + 1) % poly.len()];
            let (da, db) = (keep(a.y), keep(b.y));
            if da >= 0.0 {
                out.push(a);
            }
            if (da >= 0.0) != (db >= 0.0) {
                out.push(a.lerp(b, da / (da - db)));
            }
        }
        out
    };
    let poly = clip(clip(t.to_vec(), &|y| y - lo), &|y| hi - y);
    if poly.is_empty() {
        return false;
    }
    let flat: Vec<Vec2> = poly.iter().map(|v| v.xz()).collect();
    let n = flat.len();
    let edge = |k: usize| (flat[k], flat[(k + 1) % n]);
    let near = (0..n).any(|k| {
        let (a, b) = edge(k);
        let ab = b - a;
        let s = if ab.length_squared() > 0.0 { ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0) } else { 0.0 };
        (a + ab * s).distance_squared(p) < r * r
    });
    // or inside it (all edges turning the same way round p)
    near || (n >= 3 && {
        let sides: Vec<f32> = (0..n).map(|k| (edge(k).1 - edge(k).0).perp_dot(p - edge(k).0)).collect();
        sides.iter().all(|&s| s >= 0.0) || sides.iter().all(|&s| s <= 0.0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat ground square of half-size `h` at height `y`, as two triangles.
    fn ground(c: Vec2, h: f32, y: f32) -> Vec<[Vec3; 3]> {
        let v = |x: f32, z: f32| Vec3::new(c.x + x, y, c.y + z);
        vec![[v(-h, -h), v(h, -h), v(h, h)], [v(-h, -h), v(h, h), v(-h, h)]]
    }

    /// An upright wall along x at `z`, from `x0` to `x1`, `top` m high.
    fn wall(x0: f32, x1: f32, z: f32, top: f32) -> Vec<[Vec3; 3]> {
        let v = |x: f32, y: f32| Vec3::new(x, y, z);
        vec![[v(x0, 0.0), v(x1, 0.0), v(x1, -top)], [v(x0, 0.0), v(x1, -top), v(x0, -top)]]
    }

    /// A walker with room stays home; one inside a wall moves just clear of it, away from the court; one on a
    /// raised floor stands on it; one with nothing under it or boxed in isn't drawn; two never share a spot.
    #[test]
    fn walkers_stand_clear_of_the_court() {
        let home = Vec3::new(10.0, 0.0, 20.0);
        let mut tris = ground(Vec2::new(10.0, 20.0), 10.0, 0.0);
        assert_eq!(Solid::new(tris.clone()).clear(home, &[]), Some(home), "room at home");
        // a low kerb is stepped over, a wall through its feet isn't
        tris.extend(wall(0.0, 20.0, 20.0, 0.2));
        assert_eq!(Solid::new(tris.clone()).clear(home, &[]), Some(home), "a kerb");
        tris.extend(wall(0.0, 20.0, 20.1, 3.0));
        let at = Solid::new(tris.clone()).clear(home, &[]).unwrap();
        assert!((at.z - 20.1).abs() >= R && at.xz().distance(home.xz()) <= R + 0.25 + 1e-4, "{at}");
        assert!(at.z > 20.1 || (PLAY - at.xz().abs()).min_element() <= 0.0, "not further onto the court: {at}");
        // a raised floor 0.4 m up, a drop of a metre
        let raised = Solid::new(ground(Vec2::new(10.0, 20.0), 3.0, -0.4));
        assert_eq!(raised.clear(home, &[]), Some(Vec3::new(10.0, -0.4, 20.0)));
        assert_eq!(Solid::new(ground(Vec2::new(10.0, 20.0), 10.0, 1.0)).clear(home, &[]), None, "nothing to stand on");
        // a wall's top isn't somewhere to stand
        let mut parapet = ground(home.xz() + Vec2::new(0.0, 3.0), 2.0, 0.0);
        parapet.extend(ground(home.xz(), 0.1, -0.5));
        assert_eq!(Solid::new(parapet).clear(home, &[]).map(|p| p.y), Some(0.0), "on the wall");
        // nor a wide one with a drop behind it: the floor before it is
        let mut roof = ground(home.xz() + Vec2::new(0.0, -3.0), 3.0, 0.5);
        roof.extend(ground(home.xz() + Vec2::new(0.0, 0.5), 0.5, -0.5));
        let at = Solid::new(roof).clear(home, &[]).unwrap();
        assert!(at.y == 0.5 && at.z < home.z, "{at}");
        // walls every half metre all round, closer than a body fits between
        let mut packed = ground(Vec2::new(10.0, 20.0), 20.0, 0.0);
        (0..80).for_each(|k| packed.extend(wall(-10.0, 30.0, k as f32 * 0.5, 3.0)));
        assert_eq!(Solid::new(packed).clear(home, &[]), None, "no room");
        // one moved off its home keeps a body away from the others; one with room at home stays even beside one
        let mut s = ground(Vec2::new(10.0, 20.0), 10.0, 0.0);
        s.extend(wall(9.5, 10.5, 20.0, 3.0));
        let other = Vec3::new(10.0, 0.0, 19.5);
        let b = Solid::new(s).clear(home, &[other]).unwrap();
        assert!(b.xz().distance(other.xz()) >= 2.0 * R, "{b}");
        assert_eq!(Solid::new(ground(Vec2::new(10.0, 20.0), 10.0, 0.0)).clear(home, &[other]), Some(home));
        // a bench at home is in the way, not stood on
        let mut bench = ground(Vec2::new(10.0, 20.0), 10.0, 0.0);
        bench.extend(ground(home.xz(), 0.3, -0.5));
        let b = Solid::new(bench).clear(home, &[]).unwrap();
        assert!(b.y == 0.0 && b.xz().distance(home.xz()) > 0.3, "{b}");
    }

    /// The cylinder test: a triangle above the head or below the knee misses, one crossing the body hits, a wall
    /// edge-on within the radius hits and one beyond it misses.
    #[test]
    fn a_body_touches_what_crosses_it() {
        let p = Vec2::ZERO;
        let flat = |y: f32| [Vec3::new(-1.0, y, -1.0), Vec3::new(1.0, y, -1.0), Vec3::new(0.0, y, 1.0)];
        assert!(!touches(&flat(-2.0), p, -HEAD, -KNEE, R), "over the head");
        assert!(!touches(&flat(-0.1), p, -HEAD, -KNEE, R), "under the knee");
        assert!(touches(&flat(-1.0), p, -HEAD, -KNEE, R), "through the chest");
        let wall = |z: f32| [Vec3::new(-1.0, 0.0, z), Vec3::new(1.0, 0.0, z), Vec3::new(0.0, -3.0, z)];
        assert!(touches(&wall(0.3), p, -HEAD, -KNEE, R));
        assert!(!touches(&wall(0.5), p, -HEAD, -KNEE, R));
    }

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");
    /// The mods repo's builds (`mods/hst-mods`, the mods repo's `out/mods`).
    const MODS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods");

    /// Every installed court mod, singles and doubles: each of its layout court's walkers is drawn at home, moved
    /// (never further than [`FAR`]) or hidden, and none drawn stands in the mod's geometry; prints what moved (skipped
    /// without the ISO or the mods repo's builds).
    #[test]
    fn every_court_mods_walkers_have_room() {
        let (Ok(mut iso), true) = (hst_data::iso::Iso::open(ISO), std::path::Path::new(MODS).is_dir()) else { return eprintln!("no ISO or no {MODS}, skipped") };
        let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
        let game = hst_data::exe::Game::new(&cnf, &bin).unwrap();
        let courts = crate::mods_court::list(std::path::Path::new(MODS));
        let (mut moved, mut hidden, mut all) = (0, 0, 0);
        for c in &courts {
            let n = c.layout as usize + 1;
            let (list, plants) = crate::court_layout(&mut iso, n).unwrap();
            for singles in [true, false] {
                let players = if singles { 2 } else { 4 };
                let solid = Solid::new(crate::court_mod::solid(c, singles));
                assert!(!solid.tris.is_empty(), "{}: no triangles", c.id);
                let mut taken: Vec<Vec3> = crate::court_mod::umpire_seat(c, singles).map(Vec3::from).into_iter().collect();
                for w in hst_sim::npc::spawn(&list, &plants, &game.npc_roster(n as u32), &game.walkers(n as u32), players).into_iter().filter(|w| matches!(w.kind, hst_sim::npc::Kind::Walker(_))) {
                    let home = Vec3::new(w.world[3][0], w.world[3][1], w.world[3][2]);
                    all += 1;
                    let at = solid.clear(home, &taken);
                    let was = if solid.ground(home.xz(), home.y).is_some_and(|y| !solid.blocked(Vec3::new(home.x, y, home.z))) { "room" } else { "blocked" };
                    match at {
                        Some(a) if a == home => {}
                        Some(a) => {
                            moved += 1;
                            assert!(a.xz().distance(home.xz()) <= FAR + 1e-3 && !solid.blocked(a));
                            eprintln!("{} {}: walker at {home:.2} ({was}) -> {a:.2} ({:.2} m)", c.id, if singles { "singles" } else { "doubles" }, a.distance(home));
                        }
                        None => {
                            hidden += 1;
                            eprintln!("{} {}: walker at {home:.2} ({was}) hidden", c.id, if singles { "singles" } else { "doubles" });
                        }
                    }
                    taken.extend(at);
                }
            }
        }
        eprintln!("{} court mods: {all} walkers, {moved} moved, {hidden} hidden", courts.len());
    }
}
