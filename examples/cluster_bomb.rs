extern crate rand;
extern crate rayon;
extern crate specs;

use rand::Rand;
use rand::distributions::{IndependentSample, Range};

use rayon::iter::ParallelIterator;

use specs::{Component, DenseVecStorage, DispatcherBuilder, Entities, Fetch, HashMapStorage, Join,
            LazyUpdate, ParJoin, ReadStorage, System, VecStorage, World, WriteStorage};

const TAU: f32 = 2. * std::f32::consts::PI;

#[derive(Debug)]
struct ClusterBomb {
    fuse: usize,
}
impl Component for ClusterBomb {
    // This uses `HashMapStorage`, because only some entities are cluster bombs.
    type Storage = HashMapStorage<Self>;
}

#[derive(Debug)]
struct Shrapnel {
    durability: usize,
}
impl Component for Shrapnel {
    // This uses `HashMapStorage`, because only some entities are shrapnels.
    type Storage = HashMapStorage<Self>;
}

#[derive(Debug, Clone)]
struct Pos(f32, f32);
impl Component for Pos {
    // This uses `VecStorage`, because all entities have a position.
    type Storage = VecStorage<Self>;
}

#[derive(Debug)]
struct Vel(f32, f32);
impl Component for Vel {
    // This uses `DenseVecStorage`, because nearly all entities have a velocity.
    type Storage = DenseVecStorage<Self>;
}

struct ClusterBombSystem;
impl<'a> System<'a> for ClusterBombSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, ClusterBomb>,
        ReadStorage<'a, Pos>,
        // Allows lazily adding and removing components to entities
        // or executing arbitrary code with world access lazily via `execute`.
        Fetch<'a, LazyUpdate>,
    );

    fn run(&mut self, (entities, mut bombs, positions, updater): Self::SystemData) {
        let durability_range = Range::new(10, 20);
        // Join components in potentially parallel way using rayon.
        (&*entities, &mut bombs, &positions).par_join().for_each(
            |(entity, bomb, position)| if bomb.fuse == 0 {
                let _ = entities.delete(entity);
                for _ in 0..9 {
                    let shrapnel = entities.create();
                    updater.insert(
                        shrapnel,
                        Shrapnel {
                            durability: durability_range.ind_sample(&mut rand::thread_rng()),
                        },
                    );
                    updater.insert(shrapnel, position.clone());
                    let angle = f32::rand(&mut rand::thread_rng()) * TAU;
                    updater.insert(shrapnel, Vel(angle.sin(), angle.cos()));
                }
            } else {
                bomb.fuse -= 1;
            },
        );
    }
}

struct PhysicsSystem;
impl<'a> System<'a> for PhysicsSystem {
    type SystemData = (WriteStorage<'a, Pos>, ReadStorage<'a, Vel>);

    fn run(&mut self, (mut pos, vel): Self::SystemData) {
        (&mut pos, &vel).par_join().for_each(|(pos, vel)| {
            pos.0 += vel.0;
            pos.1 += vel.1;
        });
    }
}

struct ShrapnelSystem;
impl<'a> System<'a> for ShrapnelSystem {
    type SystemData = (Entities<'a>, WriteStorage<'a, Shrapnel>);

    fn run(&mut self, (entities, mut shrapnels): Self::SystemData) {
        (&*entities, &mut shrapnels).par_join().for_each(
            |(entity, shrapnel)| if shrapnel.durability == 0 {
                let _ = entities.delete(entity);
            } else {
                shrapnel.durability -= 1;
            },
        );
    }
}

fn main() {
    let mut world = World::new();
    world.register::<Pos>();
    world.register::<Vel>();
    world.register::<Shrapnel>();
    world.register::<ClusterBomb>();

    world
        .create_entity()
        .with(Pos(0., 0.))
        .with(ClusterBomb { fuse: 3 })
        .build();

    let mut dispatcher = DispatcherBuilder::new()
        .add(PhysicsSystem, "physics", &[])
        .add(ClusterBombSystem, "cluster_bombs", &[])
        .add(ShrapnelSystem, "shrapnels", &[])
        .build();

    let mut step = 0;
    loop {
        step += 1;
        let mut entities = 0;
        {
            // Simple console rendering
            let positions = world.read::<Pos>();
            const WIDTH: usize = 10;
            const HEIGHT: usize = 10;
            const SCALE: f32 = 1. / 4.;
            let mut screen = [[0; WIDTH]; HEIGHT];
            for entity in world.entities().join() {
                if let Some(pos) = positions.get(entity) {
                    let x = (pos.0 * SCALE + WIDTH as f32 / 2.).floor() as usize;
                    let y = (pos.1 * SCALE + HEIGHT as f32 / 2.).floor() as usize;
                    if x < WIDTH && y < HEIGHT {
                        screen[x][y] += 1;
                    }
                }
                entities += 1;
            }
            println!("Step: {}, Entities: {}", step, entities);
            for row in &screen {
                for cell in row {
                    print!("{}", cell);
                }
                println!();
            }
            println!();
        }
        if entities == 0 {
            break;
        }

        dispatcher.dispatch(&world.res);

        // Maintain dynamically added and removed entities in dispatch.
        // This is what actually executes changes done by `LazyUpdate`.
        world.maintain();
    }
}
