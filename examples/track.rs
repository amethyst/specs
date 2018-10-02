extern crate hibitset;
extern crate shrev;
extern crate specs;

use specs::prelude::*;

struct TrackedComponent(u64);

impl Component for TrackedComponent {
    type Storage = FlaggedStorage<Self>;
}

#[derive(Default)]
struct SysA {
    modified_id: Option<ReaderId<ModifiedFlag>>,
    modified: BitSet,
}

impl<'a> System<'a> for SysA {
    type SystemData = (Entities<'a>, ReadStorage<'a, TrackedComponent>);

    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);
        self.modified_id = Some(WriteStorage::<TrackedComponent>::fetch(&res).track_modified());
    }

    fn run(&mut self, (entities, tracked): Self::SystemData) {
        tracked.populate_modified(&mut self.modified_id.as_mut().unwrap(), &mut self.modified);

        for (entity, _tracked, _) in (&entities, &tracked, &self.modified).join() {
            println!("modified: {:?}", entity);
        }
    }
}

#[derive(Default)]
struct SysB;
impl<'a> System<'a> for SysB {
    type SystemData = (Entities<'a>, WriteStorage<'a, TrackedComponent>);
    fn run(&mut self, (entities, mut tracked): Self::SystemData) {
        for (entity, mut restricted) in (&entities, &mut tracked.restrict_mut()).join() {
            if entity.id() % 2 == 0 {
                let mut comp = restricted.get_mut_unchecked();
                comp.0 += 1;
            }
        }
    }
}

fn main() {
    let mut world = World::new();

    let mut dispatcher = DispatcherBuilder::new()
        .with(SysA::default(), "sys_a", &[])
        .with(SysB::default(), "sys_b", &[])
        .build();

    dispatcher.setup(&mut world.res);

    for _ in 0..10000 {
        world.create_entity().with(TrackedComponent(0)).build();
    }

    dispatcher.dispatch(&mut world.res);
    world.maintain();

    dispatcher.dispatch(&mut world.res);
    world.maintain();
}
