
extern crate specs;
extern crate shrev;
extern crate hibitset;

use specs::*;
use shrev::*;
use hibitset::*;

struct TrackedComponent(u64);
impl Component for TrackedComponent {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

#[derive(Default)]
struct SysA {
    modified_id: Option<ReaderId<ModifiedFlag>>,
    modified: BitSet,
}
impl<'a> System<'a> for SysA {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, TrackedComponent>,
    );
    fn run(&mut self, (entities, tracked): Self::SystemData) {
        let reader_id = match self.modified_id {
            Some(ref mut id) => id,
            None => {
                self.modified_id = Some(tracked.track_modified());
                self.modified_id.as_mut().unwrap()
            }
        };

        tracked.populate_modified(reader_id, &mut self.modified);

        for (entity, _tracked, _) in (&*entities, &tracked, &self.modified).join() {
            println!("modified: {:?}", entity);
        }
    }
}

#[derive(Default)]
struct SysB;
impl<'a> System<'a> for SysB {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, TrackedComponent>,
    );
    fn run(&mut self, (entities, mut tracked): Self::SystemData) {
        for (entity, (entry, restricted)) in (&*entities, &mut tracked.restrict()).join() {
            if entity.id() % 2 == 0 {
                let mut comp = restricted.get_mut_unchecked(&entry);
                comp.0 += 1;
            }
        }
    }
}

fn main() {
    let mut world = World::new();
    world.register::<TrackedComponent>();

    for _ in 0..10000 {
        world.create_entity()
            .with(TrackedComponent(0))
            .build();
    }

    let mut dispatcher = DispatcherBuilder::new()
        .add(SysA::default(), "sys_a", &[])
        .add(SysB::default(), "sys_b", &[])
        .build();

    dispatcher.dispatch(&mut world.res);
    world.maintain();

    dispatcher.dispatch(&mut world.res);
    world.maintain();
}

