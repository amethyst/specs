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
    reader_id: Option<ReaderId<ComponentEvent>>,
    inserted: BitSet,
    modified: BitSet,
    removed: BitSet,
}

impl<'a> System<'a> for SysA {
    type SystemData = (Entities<'a>, ReadStorage<'a, TrackedComponent>);

    fn setup(&mut self, res: &mut World) {
        Self::SystemData::setup(res);
        self.reader_id = Some(WriteStorage::<TrackedComponent>::fetch(&res).register_reader());
    }

    fn run(&mut self, (entities, tracked): Self::SystemData) {
        self.modified.clear();
        self.inserted.clear();
        self.removed.clear();

        let events = tracked
            .channel()
            .read(self.reader_id.as_mut().expect("ReaderId not found"));
        for event in events {
            match event {
                ComponentEvent::Modified(id) => {
                    self.modified.add(*id);
                }
                ComponentEvent::Inserted(id) => {
                    self.inserted.add(*id);
                }
                ComponentEvent::Removed(id) => {
                    self.removed.add(*id);
                }
            }
        }

        for (entity, _tracked, _) in (&entities, &tracked, &self.modified).join() {
            println!("modified: {:?}", entity);
        }

        for (entity, _tracked, _) in (&entities, &tracked, &self.inserted).join() {
            println!("inserted: {:?}", entity);
        }

        for (entity, _tracked, _) in (&entities, &tracked, &self.removed).join() {
            println!("removed: {:?}", entity);
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

    dispatcher.setup(&mut world);

    for _ in 0..50 {
        world.create_entity().with(TrackedComponent(0)).build();
    }

    dispatcher.dispatch(&mut world);
    world.maintain();

    let entities = (&world.entities(), &world.read_storage::<TrackedComponent>())
        .join()
        .map(|(e, _)| e)
        .collect::<Vec<Entity>>();
    world.delete_entities(&entities).unwrap();

    for _ in 0..50 {
        world.create_entity().with(TrackedComponent(0)).build();
    }

    dispatcher.dispatch(&mut world);
    world.maintain();
}
