extern crate hibitset;
extern crate shrev;
extern crate specs;

use specs::prelude::*;

struct TrackedComponent(u64);
impl Component for TrackedComponent {
    type Storage = FlaggedStorage<Self>;
}

struct SysA {
    modified_id: ReaderId<ModifiedFlag>,
    modified: BitSet,
}

impl SysA {
    fn new(world: &mut World) -> Self {
        let mut components = world.write::<TrackedComponent>();
        let readerid = components.track_modified();
        SysA {
            modified_id: readerid,
            modified: BitSet::new(),
        }
    }
}

impl<'a> System<'a> for SysA {
    type SystemData = (Entities<'a>, ReadStorage<'a, TrackedComponent>);
    fn run(&mut self, (entities, tracked): Self::SystemData) {
        tracked.populate_modified(&mut self.modified_id, &mut self.modified);

        for (entity, _tracked, _) in (&*entities, &tracked, &self.modified).join() {
            println!("modified: {:?}", entity);
        }
    }
}

#[derive(Default)]
struct SysB;
impl<'a> System<'a> for SysB {
    type SystemData = (Entities<'a>, WriteStorage<'a, TrackedComponent>);
    fn run(&mut self, (entities, mut tracked): Self::SystemData) {
        for (entity, comp) in (&*entities, &mut tracked).join() {
            if entity.id() % 2 == 0 {
                comp.0 += 1;
            }
        }
    }
}

fn main() {
    let mut world = World::new();
    world.register::<TrackedComponent>();

    let sysa = SysA::new(&mut world);

    for _ in 0..10000 {
        world.create_entity().with(TrackedComponent(0)).build();
    }

    let mut dispatcher = DispatcherBuilder::new()
        .with(sysa, "sys_a", &[])
        .with(SysB::default(), "sys_b", &[])
        .build();

    dispatcher.dispatch(&mut world.res);
    world.maintain();

    dispatcher.dispatch(&mut world.res);
    world.maintain();
}
