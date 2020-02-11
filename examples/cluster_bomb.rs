use rand::prelude::*;

#[cfg(feature = "parallel")]
use rayon::iter::ParallelIterator;

use specs::{prelude::*, storage::HashMapStorage, WorldExt};

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
        Read<'a, LazyUpdate>,
    );

    fn run(&mut self, (entities, mut bombs, positions, updater): Self::SystemData) {
        use rand::distributions::Uniform;

        let durability_range = Uniform::new(10, 20);
        let update_position = |(entity, bomb, position): (Entity, &mut ClusterBomb, &Pos)| {
            let mut rng = rand::thread_rng();

            if bomb.fuse == 0 {
                let _ = entities.delete(entity);
                for _ in 0..9 {
                    let shrapnel = entities.create();
                    updater.insert(
                        shrapnel,
                        Shrapnel {
                            durability: durability_range.sample(&mut rng),
                        },
                    );
                    updater.insert(shrapnel, position.clone());
                    let angle: f32 = rng.gen::<f32>() * TAU;
                    updater.insert(shrapnel, Vel(angle.sin(), angle.cos()));
                }
            } else {
                bomb.fuse -= 1;
            }
        };

        // Join components in potentially parallel way using rayon.
        #[cfg(not(feature = "parallel"))]
        {
            (&entities, &mut bombs, &positions)
                .join()
                .for_each(update_position);
        }
        #[cfg(feature = "parallel")]
        {
            (&entities, &mut bombs, &positions)
                .par_join()
                .for_each(update_position);
        }
    }
}

struct PhysicsSystem;
impl<'a> System<'a> for PhysicsSystem {
    type SystemData = (WriteStorage<'a, Pos>, ReadStorage<'a, Vel>);

    fn run(&mut self, (mut pos, vel): Self::SystemData) {
        #[cfg(not(feature = "parallel"))]
        (&mut pos, &vel).join().for_each(|(pos, vel)| {
            pos.0 += vel.0;
            pos.1 += vel.1;
        });
        #[cfg(feature = "parallel")]
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
        #[cfg(not(feature = "parallel"))]
        (&entities, &mut shrapnels)
            .join()
            .for_each(|(entity, shrapnel)| {
                if shrapnel.durability == 0 {
                    let _ = entities.delete(entity);
                } else {
                    shrapnel.durability -= 1;
                }
            });

        #[cfg(feature = "parallel")]
        (&entities, &mut shrapnels)
            .par_join()
            .for_each(|(entity, shrapnel)| {
                if shrapnel.durability == 0 {
                    let _ = entities.delete(entity);
                } else {
                    shrapnel.durability -= 1;
                }
            });
    }
}

fn main() {
    let mut world = World::new();

    let mut dispatcher = DispatcherBuilder::new()
        .with(PhysicsSystem, "physics", &[])
        .with(ClusterBombSystem, "cluster_bombs", &[])
        .with(ShrapnelSystem, "shrapnels", &[])
        .build();

    dispatcher.setup(&mut world);

    world
        .create_entity()
        .with(Pos(0., 0.))
        .with(ClusterBomb { fuse: 3 })
        .build();

    let mut step = 0;
    loop {
        step += 1;
        let mut entities = 0;
        {
            // Simple console rendering
            let positions = world.read_storage::<Pos>();
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

        dispatcher.dispatch(&world);

        // Maintain dynamically added and removed entities in dispatch.
        // This is what actually executes changes done by `LazyUpdate`.
        world.maintain();
    }
}
