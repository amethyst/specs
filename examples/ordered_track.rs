extern crate hibitset;
extern crate shrev;
extern crate specs;

use std::collections::HashMap;

use specs::prelude::*;

struct TrackedComponent(u64);

impl Component for TrackedComponent {
    type Storage = FlaggedStorage<Self>;
}

#[derive(Default)]
struct SysA {
    reader_id: Option<ReaderId<ComponentEvent>>,
    cache: HashMap<u32, (Entity, u64)>,
}

impl<'a> System<'a> for SysA {
    type SystemData = (Entities<'a>, ReadStorage<'a, TrackedComponent>);

    fn setup(&mut self, res: &mut World) {
        Self::SystemData::setup(res);
        self.reader_id = Some(WriteStorage::<TrackedComponent>::fetch(&res).register_reader());
    }

    fn run(&mut self, (entities, tracked): Self::SystemData) {
        let events = tracked
            .channel()
            .read(self.reader_id.as_mut().expect("ReaderId not found"));

        // These events are received in the same order they were operated on in the last
        // frame. However, be careful. Just because you received a
        // `Modified/Inserted` event does not mean that the entity at that index
        // has a component. To get the current state of the entity, you should replay
        // the events in order to see the final result of the component. Partial
        // iteration over the events might lead to weird bugs and issues.
        for event in events {
            match event {
                ComponentEvent::Modified(id) => {
                    let entity = entities.entity(*id);
                    if let Some(component) = tracked.get(entity) {
                        // This is safe because it can only occur after an `Inserted` event, not a
                        // `Removed` event.
                        *self.cache.get_mut(id).unwrap() = (entity, component.0);
                        println!("{:?} was changed to {:?}", entity, component.0);
                    } else {
                        println!(
                            "{:?} was changed, but was removed before the next update.",
                            entity
                        );
                    }
                }
                ComponentEvent::Inserted(id) => {
                    let entity = entities.entity(*id);
                    if let Some(component) = tracked.get(entity) {
                        self.cache.insert(*id, (entity, component.0));
                        println!("{:?} had {:?} inserted", entity, component.0);
                    } else {
                        println!(
                            "{:?} had a component inserted, but was removed before the next update.",
                            entity
                        );
                    }
                }
                ComponentEvent::Removed(id) => {
                    let entity = entities.entity(*id);
                    self.cache.remove(id);
                    println!("{:?} had its component removed", entity);
                }
            }
        }
    }
}

fn main() {
    let mut world = World::new();

    let mut dispatcher = DispatcherBuilder::new()
        .with(SysA::default(), "sys_a", &[])
        .build();

    dispatcher.setup(&mut world);

    let e1 = world.create_entity().with(TrackedComponent(1)).build();
    let e2 = world.create_entity().with(TrackedComponent(2)).build();
    let e3 = world.create_entity().with(TrackedComponent(3)).build();
    let e4 = world.create_entity().with(TrackedComponent(4)).build();

    dispatcher.dispatch(&mut world);
    world.maintain();

    {
        let mut tracked = world.write_storage::<TrackedComponent>();
        tracked.get_mut(e1).unwrap().0 = 0;
        tracked.get_mut(e2).unwrap().0 = 50;
        tracked.get_mut(e4).unwrap().0 *= 2;
        tracked.remove(e1);
    }

    dispatcher.dispatch(&mut world);
    world.maintain();

    {
        let mut tracked = world.write_storage::<TrackedComponent>();

        // Note that any removal after a modification won't be seen in the next frame,
        // instead you will find no component or if a new component was inserted
        // right after it was inserted then then you will find the new inserted
        // component rather than the modified one from earlier.
        tracked.get_mut(e3).unwrap().0 = 20;
        tracked.remove(e3);
        tracked.insert(e3, TrackedComponent(10)).unwrap();
    }

    dispatcher.dispatch(&mut world);
    world.maintain();
}
