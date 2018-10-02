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
    events: Vec<ComponentEvent>,
    inserted: BitSet,
    modified: BitSet,
    removed: BitSet,
}

impl<'a> System<'a> for SysA {
    type SystemData = (Entities<'a>, ReadStorage<'a, TrackedComponent>);

    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);
        self.reader_id = Some(WriteStorage::<TrackedComponent>::fetch(&res).track());
    }

    fn run(&mut self, (entities, tracked): Self::SystemData) {
        self.events  = tracked.channel().read(self.reader_id.as_mut().expect("ReaderId not found")).map(|e| *e).collect();

        self.modified.clear();
        self.modified.extend(self.events.iter().filter_map(|event| match event {
            ComponentEvent::Modified(index) => Some(index),
            _ => None,
        }));

        self.inserted.clear();
        self.inserted.extend(self.events.iter().filter_map(|event| match event {
            ComponentEvent::Inserted(index) => Some(index),
            _ => None,
        }));

        self.removed.clear();
        self.removed.extend(self.events.iter().filter_map(|event| match event {
            ComponentEvent::Removed(index) => Some(index),
            _ => None,
        }));

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

    dispatcher.setup(&mut world.res);

    for _ in 0..50 {
        world.create_entity().with(TrackedComponent(0)).build();
    }

    dispatcher.dispatch(&mut world.res);
    world.maintain();

    let entities = (&world.entities(), &world.read_storage::<TrackedComponent>()).join().map(|(e, _)| e).collect::<Vec<Entity>>();
    world.delete_entities(&entities);

    for _ in 0..50 {
        world.create_entity().with(TrackedComponent(0)).build();
    }

    dispatcher.dispatch(&mut world.res);
    world.maintain();
}
