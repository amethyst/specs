extern crate specs;

use specs::prelude::*;

#[derive(Clone, Debug)]
struct CompInt(i8);
impl Component for CompInt {
    type Storage = VecStorage<CompInt>;
}

#[derive(Clone, Debug)]
struct CompBool(bool);
impl Component for CompBool {
    type Storage = HashMapStorage<CompBool>;
}

fn create_world() -> World {
    let mut w = World::new();

    w.register::<CompInt>();
    w.register::<CompBool>();

    w
}

#[should_panic]
#[test]
fn task_panics() {
    struct Sys;

    impl<'a> System<'a, ()> for Sys {
        type SystemData = ();

        fn work(&mut self, _: (), _: ()) {
            panic!()
        }
    }

    let mut world = create_world();
    world
        .create_entity()
        .with(CompInt(7))
        .with(CompBool(false))
        .build();

    DispatcherBuilder::new()
        .add(Sys, "s", &[])
        .build()
        .dispatch(&mut world.res, ());
}

#[test]
fn dynamic_create() {
    struct Sys;

    impl<'a> System<'a, ()> for Sys {
        type SystemData = Entities<'a>;

        fn work(&mut self, entities: Self::SystemData, _: ()) {
            entities.create();
        }
    }

    let mut world = create_world();
    let mut dispatcher = DispatcherBuilder::new().add(Sys, "s", &[]).build();

    for _ in 0..1_000 {
        dispatcher.dispatch(&mut world.res, ());
    }
}

#[test]
fn dynamic_deletion() {
    struct Sys;

    impl<'a> System<'a, ()> for Sys {
        type SystemData = Entities<'a>;

        fn work(&mut self, entities: Self::SystemData, _: ()) {
            let e = entities.create();
            entities.delete(e);
        }
    }

    let mut world = create_world();
    let mut dispatcher = DispatcherBuilder::new().add(Sys, "s", &[]).build();

    for _ in 0..1_000 {
        dispatcher.dispatch(&mut world.res, ());
    }
}

#[test]
fn dynamic_create_and_delete() {
    use specs::Entities;

    let mut world = create_world();

    {
        let entities = world.entities();
        let entities: &Entities = &*entities;
        let five: Vec<_> = entities.create_iter().take(5).collect();

        for e in five {
            entities.delete(e);
        }
    }

    world.maintain();
}

#[test]
fn mixed_create_merge() {
    use std::collections::HashSet;

    let mut world = create_world();
    let mut set = HashSet::new();

    let add = |set: &mut HashSet<Entity>, e: Entity| {
        assert!(!set.contains(&e));
        set.insert(e);
    };

    let insert = |w: &mut World, set: &mut HashSet<Entity>, cnt: usize| {
        // Check to make sure there is no conflict between create_now
        // and create_pure
        for _ in 0..10 {
            for _ in 0..cnt {
                add(set, w.create_entity().build());
                let e = w.create_entity().build();
                w.delete_entity(e);
                add(set, w.entities().create());
                //  swap order
                add(set, w.entities().create());
                add(set, w.create_entity().build());
            }
            w.maintain();
        }
    };

    insert(&mut world, &mut set, 10);
    for e in set.drain() {
        world.entities().delete(e);
    }
    insert(&mut world, &mut set, 20);
    for e in set.drain() {
        world.delete_entity(e);
    }
    insert(&mut world, &mut set, 40);
}

#[test]
fn is_alive() {
    let mut w = World::new();

    let e = w.create_entity().build();
    assert!(w.is_alive(e));
    w.delete_entity(e);
    assert!(!w.is_alive(e));

    let e2 = w.create_entity().build();
    assert!(w.is_alive(e2));
    w.entities().delete(e2);
    assert!(w.is_alive(e2));
    w.maintain();
    assert!(!w.is_alive(e2));
}

// Checks whether entities are considered dead immediately after creation
#[test]
fn stillborn_entities() {
    struct LCG(u32);
    const RANDMAX: u32 = 32767;
    impl LCG {
        fn new() -> Self {
            LCG(0xdeadbeef)
        }
        fn geni(&mut self) -> i8 {
            ((self.gen() as i32) - 0x7f) as i8
        }
        fn gen(&mut self) -> u32 {
            self.0 = self.0.wrapping_mul(214013).wrapping_add(2531011);
            self.0 % RANDMAX
        }
    }

    #[derive(Debug)]
    struct Rand {
        values: Vec<i8>,
    }

    struct SysRand(LCG);

    impl<'a> System<'a, ()> for SysRand {
        type SystemData = FetchMut<'a, Rand>;

        fn work(&mut self, mut data: Self::SystemData, _: ()) {
            let rng = &mut self.0;

            let count = (rng.gen() % 25) as usize;
            let values: &mut Vec<i8> = &mut data.values;
            values.clear();
            for _ in 0..count {
                values.push(rng.geni());
            }
        }
    }

    struct Delete;

    impl<'a> System<'a, ()> for Delete {
        type SystemData = (Entities<'a>, ReadStorage<'a, CompInt>, Fetch<'a, Rand>);

        fn work(&mut self, data: Self::SystemData, _: ()) {
            let (entities, comp_int, rand) = data;

            let mut lowest = Vec::new();
            for (&CompInt(k), entity) in (&comp_int, &*entities).join() {
                if lowest.iter().all(|&(n, _)| n >= k) {
                    lowest.push((k, entity));
                }
            }

            lowest.reverse();
            lowest.truncate(rand.values.len());
            for (_, eid) in lowest.into_iter() {
                entities.delete(eid);
            }
        }
    }

    struct Insert;

    impl<'a> System<'a, ()> for Insert {
        type SystemData = (Entities<'a>, WriteStorage<'a, CompInt>, Fetch<'a, Rand>);

        fn work(&mut self, data: Self::SystemData, _: ()) {
            let (entities, mut comp_int, rand) = data;

            for &i in rand.values.iter() {
                use specs::InsertResult::EntityIsDead;

                let result = comp_int.insert(entities.create(), CompInt(i));
                if let EntityIsDead(_) = result {
                    panic!("Couldn't insert {} into a stillborn entity", i);
                }
            }
        }
    }

    let mut rng = LCG::new();

    // Construct a bunch of entities

    let mut world = create_world();
    world.add_resource(Rand { values: Vec::new() });

    for _ in 0..100 {
        world.create_entity().with(CompInt(rng.geni())).build();
    }

    let mut dispatcher = DispatcherBuilder::new()
        .add(SysRand(rng), "rand", &[])
        .add(Delete, "del", &["rand"])
        .add(Insert, "insert", &["del"])
        .build();

    for _ in 0..100 {
        dispatcher.dispatch(&mut world.res, ());
    }
}


#[test]
fn dynamic_component() {
    // a simple test for the dynamic component feature.
    let mut w = World::new();

    w.register_with_id::<CompInt, _>(1);
    w.register_with_id::<CompBool, _>(2);

    let e = w.create_entity()
        .with_id::<CompInt, _>(CompInt(10), 1)
        .with_id::<CompBool, _>(CompBool(true), 2)
        .build();

    let i = w.read_with_id::<CompInt, _>(1).get(e).unwrap().0;
    assert_eq!(i, 10);

    let c = w.read_with_id::<CompBool, _>(2).get(e).unwrap().0;
    assert_eq!(c, true);
}

#[test]
fn register_idempotency() {
    // Test that repeated calls to `register` do not silently
    // stomp over the existing storage, but instead silently do nothing.
    let mut w = World::new();
    w.register::<CompInt>();

    let e = w.create_entity().with::<CompInt>(CompInt(10)).build();

    // At the time this test was written, a call to `register`
    // would blindly plough ahead and stomp the existing storage, so...
    w.register::<CompInt>();

    // ...this would end up trying to unwrap a `None`.
    let i = w.read::<CompInt>().get(e).unwrap().0;
    assert_eq!(i, 10);
}
