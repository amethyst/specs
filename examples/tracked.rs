extern crate specs;

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use specs::{Change, Component, Entities, Join, RunNow, System, TrackedStorage, World, WriteStorage};

#[derive(Debug)]
struct SuperExpensive(Transform);

struct Sys {
    expensive: HashMap<u32, SuperExpensive>,
}

impl<'a> System<'a> for Sys {
    type SystemData = (Entities<'a>, WriteStorage<'a, Transform>);

    fn run(&mut self, (entities, mut transforms): Self::SystemData) {
        transforms.maintain_tracked();

        let changes = transforms.change_events_tracked();

        changes
            .iter()
            .enumerate()
            .filter(|&(_, &c)| c == Change::Removed)
            .map(|(id, _)| id as u32)
            .for_each(|id| {
                self.expensive.remove(&id);
            });

        for (entity, transform) in (&*entities, &transforms).join() {
            let id = entity.id();
            let change = changes[id as usize];

            let expensive = self.expensive.entry(id);

            let expensive = match change {
                Change::Inserted | Change::Modified => {
                    let calculated = calc(*transform);

                    insert(expensive, calculated)
                }
                Change::Removed => unreachable!(),
                Change::None => {
                    // The system cache may not be accurate,
                    // e.g. because it has been added after
                    // a reset.
                    // Thus, in case the entry is vacant,
                    // we want to do an insertion.

                    expensive.or_insert_with(|| calc(*transform))
                }
            };

            // Do something with the value
            println!("Using the value: {:?}", expensive);
        }
    }
}

struct ResetTransforms;

impl<'a> System<'a> for ResetTransforms {
    type SystemData = WriteStorage<'a, Transform>;

    fn run(&mut self, mut transforms: Self::SystemData) {
        // You should reset your transforms at the end of every
        // frame.
        // Whether you do it in the system checking the changes
        // or in a separate one depends on your architecture.
        // E.g. maybe multiple systems need change events,
        // in which case you don't want one of them to reset them
        // before all of them could query them.
        transforms.reset_tracked();
    }
}

/// Performs an expensive calculation
fn calc(transform: Transform) -> SuperExpensive {
    println!("Need to recalculate");

    SuperExpensive(transform)
}

/// Helper function to insert regardless of value
fn insert(e: Entry<u32, SuperExpensive>, val: SuperExpensive) -> &mut SuperExpensive {
    match e {
        Entry::Vacant(v) => v.insert(val),
        Entry::Occupied(mut o) => {
            o.insert(val);

            o.into_mut()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Transform {
    mat: [f32; 16],
}

impl Transform {
    fn new() -> Self {
        #![cfg_attr(rustfmt, rustfmt_skip)]

        Transform {
            mat: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }
}

impl Component for Transform {
    type Storage = TrackedStorage<Self>;
}

fn main() {
    let mut world = World::new();
    world.register::<Transform>();

    world.create_entity().with(Transform::new()).build();
    let b = world.create_entity().with(Transform::new()).build();

    let mut sys = Sys {
        expensive: HashMap::new(),
    };
    let mut reset = ResetTransforms;
    sys.run_now(&world.res);
    reset.run_now(&world.res);

    {
        let mut transform = world.write::<Transform>();
        let transform = transform.get_mut(b).unwrap();
        transform.mat[2] = 0.5;
    }

    // This time, only `SuperExpensive` for `b` needs to be recalculated.
    sys.run_now(&world.res);
    reset.run_now(&world.res);
}
