#![feature(test)]

extern crate cgmath;
extern crate rand;
extern crate specs;
extern crate test;

use cgmath::Vector2;
use rand::thread_rng;
use specs::{Component, DenseVecStorage, DispatcherBuilder, Entities, Entity, Fetch,
            HashMapStorage, Join, NullStorage, ReadStorage, RunningTime, System, VecStorage,
            World, WriteStorage};
use test::Bencher;

type Vec2 = Vector2<f32>;

// -- Components --

#[derive(Clone, Copy, Debug)]
struct Pos(Vec2);

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Copy, Debug)]
struct Vel(Vec2);

impl Component for Vel {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Copy, Debug)]
struct Force(Vec2);

impl Component for Force {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Copy, Debug)]
struct InvMass(f32);

impl Component for InvMass {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Copy, Debug)]
struct Lifetime(f32);

impl Component for Lifetime {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Copy, Debug)]
struct Ball {
    radius: f32,
}

impl Component for Ball {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    a: f32,
    b: f32,
}

impl Component for Rect {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Copy, Debug)]
enum Spawner {
    Ball { radius: f32, inv_mass: f32 },
    Rect { a: f32, b: f32, inv_mass: f32 },
}

impl Component for Spawner {
    type Storage = HashMapStorage<Self>;
}

#[derive(Clone, Copy, Debug)]
struct SpawnRequests(usize);

impl Component for SpawnRequests {
    type Storage = HashMapStorage<Self>;
}

#[derive(Clone, Copy, Debug)]
struct Collision {
    a: Entity,
    b: Entity,
    contact: Vec2,
}

impl Component for Collision {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone, Copy, Debug)]
struct Room {
    inner_width: f32,
    inner_height: f32,
}

impl Component for Room {
    type Storage = HashMapStorage<Self>;
}

#[derive(Clone, Copy, Debug)]
enum Color {
    Green,
    Red,
}

impl Component for Color {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Copy, Debug, Default)]
struct KillsEnemy;

impl Component for KillsEnemy {
    type Storage = NullStorage<Self>;
}

// -- Resources --

#[derive(Clone, Copy, Debug)]
struct DeltaTime(f32);

// -- Systems --

struct Integrate;

impl<'a> System<'a> for Integrate {
    type SystemData = (
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Force>,
        ReadStorage<'a, InvMass>,
        Fetch<'a, DeltaTime>,
    );

    fn run(&mut self, (mut pos, mut vel, mut force, inv_mass, delta): Self::SystemData) {
        use cgmath::Zero;

        let delta: f32 = delta.0;

        for (pos, vel, force, inv_mass) in (&mut pos, &mut vel, &mut force, &inv_mass).join() {
            pos.0 += vel.0 * delta;

            let damping = (0.9f32).powf(delta);
            vel.0 += force.0 * inv_mass.0;
            vel.0 *= damping;

            force.0 = Vec2::zero();
        }
    }
}

struct Spawn;

impl<'a> System<'a> for Spawn {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Spawner>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Force>,
        WriteStorage<'a, InvMass>,
        WriteStorage<'a, Ball>,
        WriteStorage<'a, Rect>,
        WriteStorage<'a, Color>,
        WriteStorage<'a, SpawnRequests>,
    );

    fn run(
        &mut self,
        (
            entities,
            spawner,
            mut pos,
            mut vel,
            mut force,
            mut inv_mass,
            mut ball,
            mut rect,
            mut color,
            mut requests
        ): Self::SystemData
){
        use cgmath::Zero;
        use rand::Rng;

        let mut rng = thread_rng();
        let mut gen = || rng.gen_range(-4.0, 4.0);

        let mut spawns = Vec::new();

        for (spawner, pos, color, requests) in (&spawner, &pos, &color, &mut requests).join() {
            for _ in 0..requests.0 {
                let spawn_pos = Vec2::new(gen(), gen());
                let spawn_pos = pos.0 + spawn_pos;

                spawns.push((*spawner, spawn_pos, *color));
            }

            requests.0 = 0;
        }

        for (spawner, spawn_pos, spawn_color) in spawns {
            let entity = entities.create();

            let spawn_inv_mass = match spawner {
                Spawner::Rect { a, b, inv_mass } => {
                    rect.insert(entity, Rect { a, b });

                    inv_mass
                }
                Spawner::Ball { radius, inv_mass } => {
                    ball.insert(entity, Ball { radius });

                    inv_mass
                }
            };

            inv_mass.insert(entity, InvMass(spawn_inv_mass));

            pos.insert(entity, Pos(spawn_pos));
            vel.insert(entity, Vel(Vec2::new(gen(), gen())));
            force.insert(entity, Force(Vec2::zero()));
            color.insert(entity, spawn_color);
        }
    }
}

struct RequestSpawns;

impl<'a> System<'a> for RequestSpawns {
    type SystemData = WriteStorage<'a, SpawnRequests>;

    fn run(&mut self, mut data: Self::SystemData) {
        use rand::Rng;

        let mut rng = thread_rng();

        for requests in (&mut data).join() {
            let num = rng.gen_range(0, 200);
            if num > 197 {
                requests.0 = 2;
            }
        }
    }
}

struct GenCollisions;

impl<'a> System<'a> for GenCollisions {
    type SystemData = (
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Ball>,
        ReadStorage<'a, Rect>,
        WriteStorage<'a, Collision>,
    );

    fn run(&mut self, _: Self::SystemData) {
        // TODO
    }

    fn running_time(&self) -> RunningTime {
        RunningTime::VeryShort
    }
}

#[bench]
fn bench_parallel(b: &mut Bencher) {
    let mut w = World::new();

    w.register::<Pos>();
    w.register::<Vel>();
    w.register::<Force>();
    w.register::<InvMass>();
    w.register::<Color>();
    w.register::<Lifetime>();
    w.register::<Ball>();
    w.register::<Rect>();
    w.register::<Room>();
    w.register::<Spawner>();
    w.register::<SpawnRequests>();
    w.register::<Collision>();

    w.add_resource(DeltaTime(0.02));

    for x in -50i32..50i32 {
        for y in -50i32..50i32 {
            let x = x as f32 * 35.0;
            let y = y as f32 * 30.0;
            let width = 30.0;
            let height = 25.0;

            let ball_spawner = Spawner::Ball {
                radius: 1.0,
                inv_mass: 2.0,
            };
            let rect_spawner = Spawner::Rect {
                a: 1.0,
                b: 3.0,
                inv_mass: 5.0,
            };

            let pos_x = [x - 8.0, x - 8.0, x + 8.0, x + 8.0];
            let pos_y = [y + 3.0, y - 3.0, y + 3.0, y - 3.0];
            let color = [Color::Green, Color::Green, Color::Red, Color::Red];
            let spawner = [ball_spawner, rect_spawner, ball_spawner, rect_spawner];

            w.create_entity()
                .with(Pos(Vec2::new(x, y)))
                .with(Room {
                    inner_width: width,
                    inner_height: height,
                })
                .build();

            for i in 0..4 {
                w.create_entity()
                    .with(Pos(Vec2::new(pos_x[i], pos_y[i])))
                    .with(spawner[i])
                    .with(SpawnRequests(0))
                    .with(Rect { a: 2.5, b: 2.5 })
                    .with(color[i])
                    .build();
            }
        }
    }

    let mut d = DispatcherBuilder::new()
        .add(RequestSpawns, "req_spawns", &[])
        .add(GenCollisions, "gen_collisions", &[])
        .add(Spawn, "spawn", &[])
        .add(Integrate, "integrate", &[])
        .build();

    b.iter(|| {
        d.dispatch(&mut w.res);
        w.maintain();
    })
}
