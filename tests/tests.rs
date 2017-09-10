extern crate rayon;
extern crate specs;

use specs::{Component, DispatcherBuilder, Entities, Entity, Fetch, FetchMut, HashMapStorage,
            InsertResult, Join, ParJoin, ReadStorage, System, VecStorage, World, WriteStorage};

#[derive(Clone, Debug)]
struct CompInt(i8);

impl Component for CompInt {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug)]
struct CompBool(bool);

impl Component for CompBool {
    type Storage = HashMapStorage<Self>;
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

    impl<'a> System<'a> for Sys {
        type SystemData = ();

        fn run(&mut self, _: ()) {
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
        .dispatch(&mut world.res);
}

#[test]
fn dynamic_create() {
    struct Sys;

    impl<'a> System<'a> for Sys {
        type SystemData = Entities<'a>;

        fn run(&mut self, entities: Self::SystemData) {
            entities.create();
        }
    }

    let mut world = create_world();
    let mut dispatcher = DispatcherBuilder::new().add(Sys, "s", &[]).build();

    for _ in 0..1_000 {
        dispatcher.dispatch(&mut world.res);
    }
}

#[test]
fn dynamic_deletion() {
    struct Sys;

    impl<'a> System<'a> for Sys {
        type SystemData = Entities<'a>;

        fn run(&mut self, entities: Self::SystemData) {
            let e = entities.create();
            entities.delete(e).unwrap();
        }
    }

    let mut world = create_world();
    let mut dispatcher = DispatcherBuilder::new().add(Sys, "s", &[]).build();

    for _ in 0..1_000 {
        dispatcher.dispatch(&mut world.res);
    }
}

#[test]
fn dynamic_create_and_delete() {
    let mut world = create_world();

    {
        let entities = world.entities();
        let entities = &*entities;
        let five: Vec<_> = entities.create_iter().take(5).collect();

        for e in five {
            entities.delete(e).unwrap();
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
                w.delete_entity(e).unwrap();
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
        world.entities().delete(e).unwrap();
    }
    insert(&mut world, &mut set, 20);
    for e in set.drain() {
        world.delete_entity(e).unwrap();
    }
    insert(&mut world, &mut set, 40);
}

#[test]
fn is_alive() {
    let mut w = World::new();

    let e = w.create_entity().build();
    assert!(w.is_alive(e));
    w.delete_entity(e).unwrap();
    assert!(!w.is_alive(e));

    let e2 = w.create_entity().build();
    assert!(w.is_alive(e2));
    w.entities().delete(e2).unwrap();
    assert!(w.is_alive(e2));
    w.maintain();
    assert!(!w.is_alive(e2));
}

// Checks whether entities are considered dead immediately after creation
#[test]
fn stillborn_entities() {
    struct LCG(u32);
    const RANDMAX: u32 = 32_767;
    impl LCG {
        fn new() -> Self {
            LCG(0xdead_beef)
        }
        fn geni(&mut self) -> i8 {
            ((self.gen() as i32) - 0x7f) as i8
        }
        fn gen(&mut self) -> u32 {
            self.0 = self.0.wrapping_mul(214_013).wrapping_add(2_531_011);
            self.0 % RANDMAX
        }
    }

    #[derive(Debug)]
    struct Rand {
        values: Vec<i8>,
    }

    struct SysRand(LCG);

    impl<'a> System<'a> for SysRand {
        type SystemData = FetchMut<'a, Rand>;

        fn run(&mut self, mut data: Self::SystemData) {
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

    impl<'a> System<'a> for Delete {
        type SystemData = (Entities<'a>, ReadStorage<'a, CompInt>, Fetch<'a, Rand>);

        fn run(&mut self, (entities, comp_int, rand): Self::SystemData) {
            let mut lowest = Vec::new();
            for (&CompInt(k), entity) in (&comp_int, &*entities).join() {
                if lowest.iter().all(|&(n, _)| n >= k) {
                    lowest.push((k, entity));
                }
            }

            lowest.reverse();
            lowest.truncate(rand.values.len());
            for (_, eid) in lowest {
                entities.delete(eid).unwrap();
            }
        }
    }

    struct Insert;

    impl<'a> System<'a> for Insert {
        type SystemData = (Entities<'a>, WriteStorage<'a, CompInt>, Fetch<'a, Rand>);

        fn run(&mut self, (entities, mut comp_int, rand): Self::SystemData) {
            for &i in &rand.values {
                let result = comp_int.insert(entities.create(), CompInt(i));
                if let InsertResult::EntityIsDead(_) = result {
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
        dispatcher.dispatch(&mut world.res);
    }
}


#[test]
fn dynamic_component() {
    // a simple test for the dynamic component feature.
    let mut w = World::new();

    w.register_with_id::<CompInt>(1);
    w.register_with_id::<CompBool>(2);

    let e = w.create_entity()
        .with_id(CompInt(10), 1)
        .with_id(CompBool(true), 2)
        .build();

    let i = w.read_with_id::<CompInt>(1).get(e).unwrap().0;
    assert_eq!(i, 10);

    let c = w.read_with_id::<CompBool>(2).get(e).unwrap().0;
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

#[test]
fn join_two_components() {
    let mut world = create_world();
    world
        .create_entity()
        .with(CompInt(1))
        .with(CompBool(false))
        .build();
    world
        .create_entity()
        .with(CompInt(2))
        .with(CompBool(true))
        .build();
    world.create_entity().with(CompInt(3)).build();

    struct Iter;
    impl<'a> System<'a> for Iter {
        type SystemData = (ReadStorage<'a, CompInt>, ReadStorage<'a, CompBool>);

        fn run(&mut self, (int, boolean): Self::SystemData) {
            let (mut first, mut second) = (false, false);
            for (int, boolean) in (&int, &boolean).join() {
                if int.0 == 1 && !boolean.0 {
                    first = true;
                } else if int.0 == 2 && boolean.0 {
                    second = true;
                } else {
                    panic!(
                        "Entity with compent values that shouldn't be: {:?} {:?}",
                        int,
                        boolean
                    );
                }
            }
            assert!(
                first,
                "There should be entity with CompInt(1) and CompBool(false)"
            );
            assert!(
                second,
                "There should be entity with CompInt(2) and CompBool(true)"
            );
        }
    }
    let mut dispatcher = DispatcherBuilder::new().add(Iter, "iter", &[]).build();
    dispatcher.dispatch(&mut world.res);
}

#[test]
fn par_join_two_components() {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    let mut world = create_world();
    world
        .create_entity()
        .with(CompInt(1))
        .with(CompBool(false))
        .build();
    world
        .create_entity()
        .with(CompInt(2))
        .with(CompBool(true))
        .build();
    world.create_entity().with(CompInt(3)).build();
    let first = AtomicBool::new(false);
    let second = AtomicBool::new(false);
    let error = Mutex::new(None);
    struct Iter<'a>(
        &'a AtomicBool,
        &'a AtomicBool,
        &'a Mutex<Option<(i8, bool)>>,
    );
    impl<'a, 'b> System<'a> for Iter<'b> {
        type SystemData = (ReadStorage<'a, CompInt>, ReadStorage<'a, CompBool>);

        fn run(&mut self, (int, boolean): Self::SystemData) {
            use rayon::iter::ParallelIterator;
            let Iter(first, second, error) = *self;
            (&int, &boolean).par_join().for_each(|(int, boolean)| {
                if !first.load(Ordering::SeqCst) && int.0 == 1 && !boolean.0 {
                    first.store(true, Ordering::SeqCst);
                } else if !second.load(Ordering::SeqCst) && int.0 == 2 && boolean.0 {
                    second.store(true, Ordering::SeqCst);
                } else {
                    *error.lock().unwrap() = Some((int.0, boolean.0));
                }
            });
        }
    }
    let mut dispatcher = DispatcherBuilder::new()
        .add(Iter(&first, &second, &error), "iter", &[])
        .build();
    dispatcher.dispatch(&mut world.res);
    assert_eq!(
        *error.lock().unwrap(),
        None,
        "Entity shouldn't be in the join",
    );
    assert!(
        first.load(Ordering::SeqCst),
        "There should be entity with CompInt(1) and CompBool(false)"
    );
    assert!(
        second.load(Ordering::SeqCst),
        "There should be entity with CompInt(2) and CompBool(true)"
    );
}

#[test]
fn par_join_many_entities_and_systems() {
    use std::sync::Mutex;
    use rayon::iter::ParallelIterator;

    let failed = Mutex::new(vec![]);
    let mut world = create_world();
    for _ in 0..1000 {
        world.create_entity().with(CompInt(-128)).build();
    }
    struct Incr;
    impl<'a> System<'a> for Incr {
        type SystemData = (Entities<'a>, WriteStorage<'a, CompInt>);

        fn run(&mut self, (entities, mut ints): Self::SystemData) {
            (&mut ints, &*entities)
                .par_join()
                .for_each(|(int, _)| { int.0 += 1; });
        }
    }
    let mut builder = DispatcherBuilder::new();
    for _ in 0..255 {
        builder = builder.add(Incr, "", &[]);
    }
    struct FindFailed<'a>(&'a Mutex<Vec<(u32, i8)>>);
    impl<'a, 'b> System<'a> for FindFailed<'b> {
        type SystemData = (Entities<'a>, ReadStorage<'a, CompInt>);

        fn run(&mut self, (entities, ints): Self::SystemData) {
            (&ints, &*entities)
                .par_join()
                .for_each(|(int, entity)| if int.0 != 127 {
                    self.0.lock().unwrap().push((entity.id(), int.0));
                });
        }
    }
    let mut dispatcher = builder
        .add_barrier()
        .add(FindFailed(&failed), "find_failed", &[])
        .build();
    dispatcher.dispatch(&mut world.res);
    for &(id, n) in &*failed.lock().unwrap() {
        panic!(
            "Entity with id {} failed to count to 127. Count was {}",
            id,
            n
        );
    }
}
