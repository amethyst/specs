extern crate parsec;

use parsec::Storage;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug)]
struct CompInt(i8);
impl parsec::Component for CompInt {
    type Storage = parsec::VecStorage<CompInt>;
}
#[derive(Clone, Debug)]
struct CompBool(bool);
impl parsec::Component for CompBool {
    type Storage = parsec::HashMapStorage<CompBool>;
}

fn create_world() -> parsec::Scheduler {
    let mut w = parsec::World::new();
    w.register::<CompInt>();
    w.register::<CompBool>();
    parsec::Scheduler::new(w, 4)
}

#[test]
fn wait() {
    let mut scheduler = create_world();

    for _ in 0..100 {
        let found_ent_0 = Arc::new(AtomicBool::new(false));
        let found_ent_1 = Arc::new(AtomicBool::new(false));

        scheduler.world.create_now()
            .with(CompInt(7))
            .with(CompBool(false))
            .build();

        let marker = found_ent_0.clone();
        scheduler.run1w1r(move |b: &mut CompBool, r: &CompInt| {
            b.0 = r.0 == 7;
            marker.store(true, Ordering::SeqCst);
        });
        let marker = found_ent_1.clone();
        scheduler.run0w2r(move |r: &CompInt, b: &CompBool| {
            assert_eq!(r.0, 7);
            assert_eq!(b.0, true);
            marker.store(true, Ordering::SeqCst);
        });
        scheduler.wait();

        assert_eq!(found_ent_0.load(Ordering::SeqCst), true);
        assert_eq!(found_ent_1.load(Ordering::SeqCst), true);
    }
}

#[should_panic]
#[test]
fn task_panics() {
    let mut scheduler = create_world();
    scheduler.world.create_now()
        .with(CompInt(7))
        .with(CompBool(false))
        .build();

    scheduler.run(|args| {
        args.fetch(|_| ());
        panic!();
    });
    scheduler.wait();
}


#[should_panic]
#[test]
fn task_panics_args_captured() {
    let mut scheduler = create_world();
    scheduler.world.create_now()
        .with(CompInt(7))
        .with(CompBool(false))
        .build();

    scheduler.run(|_| {
        panic!();
    });
    scheduler.wait();
}
