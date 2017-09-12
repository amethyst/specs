#![feature(test)]

extern crate rayon;
extern crate specs;
extern crate test;

use specs::{Component, HashMapStorage, Join, ParJoin, VecStorage, World};

#[derive(Clone, Debug)]
struct CompInt(i32);

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

#[bench]
fn world_build(b: &mut test::Bencher) {
    b.iter(World::new);
}

#[bench]
fn create_now(b: &mut test::Bencher) {
    let mut w = World::new();
    b.iter(|| w.create_entity().build());
}

#[bench]
fn create_now_with_storage(b: &mut test::Bencher) {
    let mut w = create_world();
    b.iter(|| w.create_entity().with(CompInt(0)).build());
}

#[bench]
fn create_pure(b: &mut test::Bencher) {
    let w = World::new();
    b.iter(|| w.entities().create());
}

#[bench]
fn delete_now(b: &mut test::Bencher) {
    let mut w = World::new();
    let mut eids: Vec<_> = (0..10_000_000).map(|_| w.create_entity().build()).collect();
    b.iter(|| if let Some(id) = eids.pop() {
        w.delete_entity(id).unwrap()
    });
}

#[bench]
fn delete_now_with_storage(b: &mut test::Bencher) {
    let mut w = create_world();
    let mut eids: Vec<_> = (0..10_000_000)
        .map(|_| w.create_entity().with(CompInt(1)).build())
        .collect();
    b.iter(|| if let Some(id) = eids.pop() {
        w.delete_entity(id).unwrap()
    });
}

#[bench]
fn delete_later(b: &mut test::Bencher) {
    let mut w = World::new();
    let mut eids: Vec<_> = (0..10_000_000).map(|_| w.create_entity().build()).collect();
    b.iter(|| if let Some(id) = eids.pop() {
        w.entities().delete(id).unwrap()
    });
}

#[bench]
fn maintain_noop(b: &mut test::Bencher) {
    let mut w = World::new();
    b.iter(|| { w.maintain(); });
}

#[bench]
fn maintain_add_later(b: &mut test::Bencher) {
    let mut w = World::new();
    b.iter(|| {
        w.entities().create();
        w.maintain();
    });
}

#[bench]
fn maintain_delete_later(b: &mut test::Bencher) {
    let mut w = World::new();
    let mut eids: Vec<_> = (0..10_000_000).map(|_| w.create_entity().build()).collect();
    b.iter(|| {
        if let Some(id) = eids.pop() {
            w.entities().delete(id).unwrap();
        }
        w.maintain();
    });
}

#[bench]
fn join_single_threaded(b: &mut test::Bencher) {
    use test::black_box;

    let mut world = World::new();
    world.register::<CompInt>();

    {
        let entities: Vec<_> = world.create_iter().take(50_000).collect();
        let mut comp_int = world.write();
        for (i, e) in entities.iter().enumerate() {
            comp_int.insert(*e, CompInt(i as i32));
        }
    }

    b.iter(|| for comp in world.read::<CompInt>().join() {
        black_box(comp.0 * comp.0);
    })
}

#[bench]
fn join_multi_threaded(b: &mut test::Bencher) {
    use rayon::prelude::*;
    use test::black_box;

    let mut world = World::new();
    world.register::<CompInt>();

    {
        let entities: Vec<_> = world.create_iter().take(50_000).collect();
        let mut comp_int = world.write();
        for (i, e) in entities.iter().enumerate() {
            comp_int.insert(*e, CompInt(i as i32));
        }
    }

    b.iter(|| {
        world
            .read::<CompInt>()
            .par_join()
            .for_each(|comp| { black_box(comp.0 * comp.0); })
    })
}
