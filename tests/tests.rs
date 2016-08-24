extern crate specs;

use specs::{Entity};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug)]
struct CompInt(i8);
impl specs::Component for CompInt {
    type Storage = specs::VecStorage<CompInt>;
}
#[derive(Clone, Debug)]
struct CompBool(bool);
impl specs::Component for CompBool {
    type Storage = specs::HashMapStorage<CompBool>;
}

#[cfg(feature="parallel")]
fn create_world() -> specs::Planner<()> {
    let mut w = specs::World::new();
    w.register::<CompInt>();
    w.register::<CompBool>();
    specs::Planner::new(w, 4)
}

#[test]
#[cfg(feature="parallel")]
fn wait() {
    let mut planner = create_world();

    for _ in 0..100 {
        let found_ent_0 = Arc::new(AtomicBool::new(false));
        let found_ent_1 = Arc::new(AtomicBool::new(false));

        planner.mut_world().create_now()
            .with(CompInt(7))
            .with(CompBool(false))
            .build();

        let marker = found_ent_0.clone();
        planner.run1w1r(move |b: &mut CompBool, r: &CompInt| {
            b.0 = r.0 == 7;
            marker.store(true, Ordering::SeqCst);
        });
        let marker = found_ent_1.clone();
        planner.run0w2r(move |r: &CompInt, b: &CompBool| {
            assert_eq!(r.0, 7);
            assert_eq!(b.0, true);
            marker.store(true, Ordering::SeqCst);
        });
        planner.wait();

        assert_eq!(found_ent_0.load(Ordering::SeqCst), true);
        assert_eq!(found_ent_1.load(Ordering::SeqCst), true);
    }
}

//#[should_panic]
//#[test] //TODO
#[cfg(feature="parallel")]
fn _task_panics() {
    let mut planner = create_world();
    planner.mut_world().create_now()
        .with(CompInt(7))
        .with(CompBool(false))
        .build();

    planner.run_custom(|args| {
        args.fetch(|_| ());
        panic!();
    });
    planner.wait();
}


#[should_panic]
#[test]
#[cfg(feature="parallel")]
fn task_panics_args_captured() {
    let mut planner = create_world();
    planner.mut_world().create_now()
        .with(CompInt(7))
        .with(CompBool(false))
        .build();

    planner.run_custom(|_| {
        panic!();
    });
    planner.wait();
}

#[test]
#[cfg(feature="parallel")]
fn dynamic_create() {
    let mut planner = create_world();

    for _ in 0..1_000 {
        planner.run_custom(|arg| {
            arg.fetch(|_| ());
            arg.create();
        });
        planner.wait();
    }
}

#[test]
#[cfg(feature="parallel")]
fn dynamic_deletion() {
    let mut planner = create_world();

    for _ in 0..1_000 {
        planner.run_custom(|arg| {
            arg.fetch(|_| ());
            let e = arg.create();
            arg.delete(e);
            arg.delete(e); // double free
        });
        planner.wait();
    }
}

#[test]
#[cfg(feature="parallel")]
fn dynamic_create_and_delete() {
    use std::mem::swap;
    let mut planner = create_world();

    let (mut ent0, mut ent1) = (
        Arc::new(Mutex::new(None)),
        Arc::new(Mutex::new(None))
    );

    for i in 0..1_000 {
        let e = ent0.clone();
        planner.run_custom(move |arg| {
            arg.fetch(|_| ());
            let mut e = e.lock().unwrap();
            *e = Some(arg.create());
        });
        if i >= 1 {
            let e = ent1.clone();
            planner.run_custom(move |arg| {
                arg.fetch(|_| ());
                let mut e = e.lock().unwrap();
                arg.delete(e.take().unwrap());
            })
        }
        planner.wait();
        swap(&mut ent1, &mut ent0)
    }
}

#[test]
#[cfg(feature="parallel")]
fn mixed_create_merge() {
    use std::collections::HashSet;
    let mut planner = create_world();
    let mut set = HashSet::new();

    let add = |set: &mut HashSet<Entity>, e: Entity| {
        assert!(!set.contains(&e));
        set.insert(e);
    };

    let insert = |planner: &mut specs::Planner<()>, set: &mut HashSet<Entity>, cnt: usize| {
        // Check to make sure there is no conflict between create_now
        // and create_later
        for _ in 0..10 {
            for _ in 0..cnt {
                let mut w = planner.mut_world();
                add(set, w.create_now().build());
                let e = w.create_now().build();
                w.delete_now(e);
                add(set, w.create_later());
                //  swap order
                add(set, w.create_later());
                add(set, w.create_now().build());
            }
            planner.wait();
        }
    };

    insert(&mut planner, &mut set, 10);
    for e in set.drain() {
        planner.mut_world().delete_later(e);
    }
    insert(&mut planner, &mut set, 20);
    for e in set.drain() {
        planner.mut_world().delete_now(e);
    }
    insert(&mut planner, &mut set, 40);
}

#[test]
fn is_alive() {
    let mut w = specs::World::new();

    let e = w.create_now().build();
    assert!(w.is_alive(e));
    w.delete_now(e);
    assert!(!w.is_alive(e));

    let e2 = w.create_now().build();
    assert!(w.is_alive(e2));
    w.delete_later(e2);
    assert!(w.is_alive(e2));
    w.maintain();
    assert!(!w.is_alive(e2));
}

// Checks whether entities are considered dead immediately after creation
#[test]
#[cfg(feature="parallel")]
fn stillborn_entities() {
    struct LCG(u32);
    const RANDMAX: u32 = 32767;
    impl LCG {
        fn new() -> Self { LCG(0xdeadbeef) }
        fn geni(&mut self) -> i8 { ((self.gen() as i32) - 0x7f) as i8 }
        fn gen(&mut self) -> u32 {
            self.0 = self.0.wrapping_mul(214013).wrapping_add(2531011);
            self.0 % RANDMAX
        }
    }

    let mut rng = LCG::new();

    // Construct a bunch of entities
    let mut planner = specs::Planner::<()>::new({
        let mut world = specs::World::new();
        world.register::<CompInt>();

        for _ in 0 .. 100 {
            world.create_now().with(CompInt(rng.geni())).build();
        }

        world
    }, 4);

    for _ in 0 .. 100 {
        let count = (rng.gen() % 25) as usize;
        let mut values = vec![];
        for _ in 0 .. count { values.push(rng.geni()); }

        // Cull the same number of entities we expect to insert
        planner.run_custom(move |arg| {
            use specs::Join;

            let (compint, eids) = arg.fetch(|w| {
                (w.read::<CompInt>(), w.entities())
            });

            let mut lowest = vec![];
            for (&CompInt(k), eid) in (&compint, &eids).iter() {
                if lowest.iter().all(|&(n, _)| n >= k) {
                    lowest.push((k, eid));
                }
            }

            lowest.reverse();
            lowest.truncate(count);
            for (_, eid) in lowest.into_iter() { arg.delete(eid); }
        });

        planner.run_custom(move |arg| {
            let mut compint = arg.fetch(|w| w.write::<CompInt>());

            for &i in values.iter() {
                use specs::InsertResult::EntityIsDead;

                let result = compint.insert(arg.create(), CompInt(i));
                if let EntityIsDead(_) = result {
                    panic!("Couldn't insert {} into a stillborn entity", i);
                }
            }
        });

        planner.wait();
    }
}


#[test]
fn dynamic_component() {
    // a simple test for the dynamic component feature.
    let mut w = specs::World::<i32>::new_w_comp_id();

    w.register_w_comp_id::<CompInt>(1);
    w.register_w_comp_id::<CompBool>(2);

    let e = w.create_now()
        .with_w_comp_id::<CompInt>(1, CompInt(10))
        .with_w_comp_id::<CompBool>(2, CompBool(true))
        .build();

    let i = w.read_w_comp_id::<CompInt>(1).get(e).unwrap().0;
    assert_eq!(i, 10);

    let c = w.read_w_comp_id::<CompBool>(2).get(e).unwrap().0;
    assert_eq!(c, true);
}
