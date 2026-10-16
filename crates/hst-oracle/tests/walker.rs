//! The walking spectators' wander tick (dodge, turn back, court edges, collisions) against the game's, on random
//! states of one walker in a court 4 match (its animation calls stubbed: they only record the animation picked).

use hst_oracle::{Ee, HstEe, addr};
use hst_sim::npc::{Around, Walker, Wander};
use hst_sim::vu0::V4;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
    fn f(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next() as f32 / u32::MAX as f32)
    }
    fn bit(&mut self, p: u32) -> bool {
        self.next() % 100 < p
    }
}

fn put(e: &mut Ee, a: u32, x: V4) {
    x.iter().enumerate().for_each(|(k, f)| e.write(a + 4 * k as u32, 4, f.to_bits() as u128));
}

fn v4(e: &Ee, a: u32) -> V4 {
    std::array::from_fn(|k| e.f32_at(a + 4 * k as u32))
}

#[test]
fn walkers_wander_like_the_game() {
    let Some(mut ee) = Ee::from_ram("s04.bin") else { return };
    let w = 0x18f_b260u32;
    // set animation: only note the animation (+0xc8) and that it was set; the animation tick: nothing
    let sets = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = sets.clone();
    ee.hooks.insert(
        addr("walker_set_anim"),
        Box::new(move |e: &mut Ee| {
            e.write(e.r(4) as u32 + 0xc8, 4, e.r(5) as u32 as u128);
            counted.set(counted.get() + 1);
        }),
    );
    let an = addr("walker_animate");
    ee.write(an, 4, 0x03e0_0008);
    ee.write(an + 4, 4, 0);
    let mut iso = hst_data::iso::Iso::open(hst_oracle::root().join("Hot Shots Tennis (USA).iso")).unwrap();
    let world = std::rc::Rc::new(hst_sim::court::world(&mut iso, 4));
    let hooked = world.clone();
    let walls = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let (world_hits, walls) = (walls.clone(), walls);
    // the world sweep runs VU0 microcode: the port's mesh sweep stands in for it
    ee.hooks.insert(
        addr("world_sweep"),
        Box::new(move |e: &mut Ee| {
            let (hit, start, end) = (e.r(5) as u32, e.r(6) as u32, e.r(7) as u32);
            let found = hooked.sweep(v4(e, start), v4(e, end), e.f32_at(start + 0x10), &[]);
            walls.set(walls.get() + found.is_some() as u32);
            if let Some(h) = found {
                e.write(hit, 4, h.t.to_bits() as u128);
                e.write(hit + 4, 4, h.penetration.to_bits() as u128);
                put(e, hit + 0x10, h.centre);
                put(e, hit + 0x20, h.point);
                put(e, hit + 0x30, h.normal);
            }
            e.set(2, found.is_some() as u64);
        }),
    );
    let gm = ee.read(0x42_2f80, 4) as u32;
    let ball = ee.read(gm + 0x88, 4) as u32;
    let mgr = ee.read(0x43_b1c0, 4) as u32;
    let (np, nw) = (ee.read(0x42_2fa4, 4) as usize, ee.read(mgr + 0x800, 4) as usize);
    let players: Vec<u32> = (0..np as u32)
        .map(|i| {
            let h = ee.read(gm + 0xa8 + 4 * i, 4) as u64;
            ee.call(0x34_8fe0, &[h], &[], 10_000);
            ee.r(2) as u32 + 0x30
        })
        .collect();
    let ctrl = ee.read(w + 0x60, 4) as u32;
    let slot = ee.read(w + 0xbc, 4) as u32;
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let (mut moved, mut dodged, mut hit, mut turned, mut frozen) = (0, 0, 0, 0, 0);
    for case in 0..12_000 {
        // the walker: somewhere around the court (inside, on its edges, out by the stands), facing anywhere
        let home = [rng.f(-9.0, 9.0), 0.0, rng.f(-16.0, 16.0), 1.0];
        let mut pos = home;
        if rng.bit(40) {
            // on or by an edge or corner of the court rectangle
            pos[0] = if rng.bit(50) { 5.485 } else { -5.485 } + rng.f(-0.8, 0.8);
            pos[2] = if rng.bit(30) { rng.f(-12.5, 12.5) } else { (if rng.bit(50) { 11.885 } else { -11.885 }) + rng.f(-0.8, 0.8) };
        }
        pos[1] = if rng.bit(70) { 0.0 } else { rng.f(-0.2, 0.5) };
        let m = {
            let mut m = hst_sim::npc::walker_facing(home, rng.f(-12.0, 12.0));
            m[3] = pos;
            m
        };
        for base in [0xe0, 0x160, 0x1a0] {
            (0..4).for_each(|r| put(&mut ee, w + base + 0x10 * r, m[r as usize]));
        }
        let unit = |rng: &mut Rng| {
            let a = rng.f(-3.1, 3.1);
            [a.sin(), 0.0, a.cos(), 0.0]
        };
        let near = |rng: &mut Rng, r: f32| -> V4 { [pos[0] + rng.f(-r, r), rng.f(0.0, 1.5), pos[2] + rng.f(-r, r), 1.0] };
        let mut wa = Wander {
            home,
            world: m,
            dodging: rng.bit(40),
            ticks: (rng.next() % 32) as i32,
            dir: unit(&mut rng),
            hits: (rng.next() % 5) as i32,
            window: (rng.next() % 12) as i32,
            frozen: rng.bit(25),
            turning: rng.bit(40),
            angle: rng.f(-3.2, 3.2),
            turn_step: rng.f(-0.3, 0.3),
            turn_count: (rng.next() % 12) as i32,
            push: [rng.f(-6.0, 6.0), 0.0, rng.f(-12.0, 12.0), 0.0],
            ..Default::default()
        };
        // what's around: players and walkers near or far, standing still or moving; the ball near, still or flying
        let mut ps = Vec::new();
        for (i, &pl) in players.iter().enumerate().take(np) {
            let p = if rng.bit(40) { near(&mut rng, 2.5) } else { [rng.f(-6.0, 6.0), 0.0, rng.f(-13.0, 13.0), 1.0] };
            let prev = if rng.bit(30) { p } else { [p[0] + rng.f(-0.2, 0.2), p[1], p[2] + rng.f(-0.2, 0.2), 1.0] };
            put(&mut ee, pl, p);
            put(&mut ee, w + 0x220 + 0x10 * i as u32, prev);
            ps.push(p);
            wa.players[i] = prev;
        }
        let bp = if rng.bit(50) { near(&mut rng, 3.5) } else { [rng.f(-6.0, 6.0), rng.f(0.0, 3.0), rng.f(-13.0, 13.0), 1.0] };
        let bv = if rng.bit(20) { [0.0; 4] } else { [rng.f(-0.5, 0.5), rng.f(-0.2, 0.2), rng.f(-0.8, 0.8), 0.0] };
        put(&mut ee, ball + 0xe0, bp);
        put(&mut ee, ball + 0x130, bv);
        let mut ws = Vec::new();
        for k in 0..nw {
            let p = if k as u32 == slot { pos } else if rng.bit(40) { near(&mut rng, 1.5) } else { [rng.f(-9.0, 9.0), 0.0, rng.f(-16.0, 16.0), 1.0] };
            let prev = if rng.bit(40) { p } else { [p[0] + rng.f(-0.1, 0.1), p[1], p[2] + rng.f(-0.1, 0.1), 1.0] };
            put(&mut ee, mgr + 0x6b0 + 0x40 * k as u32, p);
            put(&mut ee, w + 0x2a0 + 0x10 * k as u32, prev);
            ws.push(p);
            wa.walkers[k] = prev;
        }
        let anim = rng.next() % 5;
        let looping = rng.bit(50);
        let advancing = rng.bit(50);
        let b = |x: bool| x as u128;
        for (o, n, v) in [
            (0x260, 1, b(wa.dodging)),
            (0x264, 4, wa.ticks as u128),
            (0x280, 4, wa.hits as u128),
            (0x284, 4, wa.window as u128),
            (0x288, 1, b(wa.frozen)),
            (0x1f4, 1, b(wa.turning)),
            (0x1f0, 4, wa.angle.to_bits() as u128),
            (0x28c, 4, wa.turn_step.to_bits() as u128),
            (0x290, 4, wa.turn_count as u128),
            (0xc8, 4, anim as u128),
            (0xcc, 1, b(advancing)),
            (0xd0, 4, u32::MAX as u128),
            (0x210, 4, 0),
            (0xc4, 4, 0),
            (0x301, 1, 0),
        ] {
            ee.write(w + o, n, v);
        }
        ee.write(ctrl + 0x78, 1, b(looping));
        put(&mut ee, w + 0x270, wa.dir);
        put(&mut ee, w + 0x200, wa.push);

        let mut port = Walker { slot, counter: -1, anim, looping, advancing, lens: [100.0; 7], wander: wa.clone(), ..Default::default() };
        let mut at = Around { players: &ps, ball: bp, ball_vel: bv, walkers: &mut ws, world: &world, side: 6.4 };
        port.wander(np as u32, &mut at);
        sets.set(0);
        ee.call(addr("walker_tick"), &[w as u64], &[], 2_000_000);

        let got = &port.wander;
        let game = Wander {
            home,
            world: std::array::from_fn(|r| v4(&ee, w + 0xe0 + 0x10 * r as u32)),
            dodging: ee.read(w + 0x260, 1) != 0,
            ticks: ee.read(w + 0x264, 4) as i32,
            dir: v4(&ee, w + 0x270),
            hits: ee.read(w + 0x280, 4) as i32,
            window: ee.read(w + 0x284, 4) as i32,
            frozen: ee.read(w + 0x288, 1) != 0,
            turning: ee.read(w + 0x1f4, 1) != 0,
            angle: ee.f32_at(w + 0x1f0),
            turn_step: ee.f32_at(w + 0x28c),
            turn_count: ee.read(w + 0x290, 4) as i32,
            push: v4(&ee, w + 0x200),
            players: std::array::from_fn(|i| if i < np { v4(&ee, w + 0x220 + 0x10 * i as u32) } else { [0.0; 4] }),
            walkers: std::array::from_fn(|k| if k < nw { v4(&ee, w + 0x2a0 + 0x10 * k as u32) } else { [0.0; 4] }),
        };
        let bits = |x: &Wander| format!("{:?}", (x.world.map(|r| r.map(f32::to_bits)), x.dir.map(f32::to_bits), x.push.map(f32::to_bits), x.angle.to_bits(), x.turn_step.to_bits(), x.players.map(|r| r.map(f32::to_bits)), x.walkers.map(|r| r.map(f32::to_bits))));
        let flags = |x: &Wander| (x.dodging, x.ticks, x.hits, x.window, x.frozen, x.turning, x.turn_count);
        assert_eq!(flags(got), flags(&game), "case {case}: flags");
        assert_eq!(bits(got), bits(&game), "case {case}: from {wa:?} pos {pos:?}\n{got:?}\n vs {game:?}");
        assert_eq!(port.anim, ee.read(w + 0xc8, 4) as u32, "case {case}: animation");
        assert_eq!(at.walkers[slot as usize].map(f32::to_bits), v4(&ee, mgr + 0x6b0 + 0x40 * slot).map(f32::to_bits), "case {case}: drawn position");
        if sets.get() == 0 {
            assert_eq!(port.looping, ee.read(ctrl + 0x78, 1) != 0, "case {case}: looping");
            assert_eq!(port.advancing, ee.read(w + 0xcc, 1) != 0, "case {case}: advancing");
        }
        moved += (got.world[3] != pos) as u32;
        dodged += got.dodging as u32;
        hit += (got.push != wa.push && got.dir == got.push) as u32;
        turned += got.turning as u32;
        frozen += got.frozen as u32;
    }
    let world_hits = world_hits.get();
    println!("moved {moved} dodging {dodged} blocked {hit} (world hits {world_hits}) turning {turned} frozen {frozen}");
    assert!(moved > 1000 && hit > 50 && world_hits > 20 && frozen > 100, "moved {moved} blocked {hit} world {world_hits}");
}
