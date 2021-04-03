#![feature(test)]

#[macro_use]
extern crate criterion;
extern crate rand;
extern crate rayon;
extern crate specs;
extern crate test;

use criterion::{Bencher, Criterion};
use specs::{prelude::*, storage::HashMapStorage};

#[derive(Clone, Debug)]
struct CompInt(i32);

impl Component for CompInt {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug)]
struct CompIntTwo(i32);

impl Component for CompIntTwo {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug)]
struct CompIntThree(i32);

impl Component for CompIntThree {
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
    w.register::<CompIntTwo>();
    w.register::<CompIntThree>();
    w.register::<CompBool>();

    w
}

fn world_build(b: &mut Bencher) {
    b.iter(World::new);
}

fn create_now(b: &mut Bencher) {
    b.iter_with_large_setup(
        World::new,
        |mut w| {
            w.create_entity().build();
        },
    );
}

fn create_now_with_storage(b: &mut Bencher) {
    b.iter_with_large_setup(
        create_world,
        |mut w| {
            w.create_entity().with(CompInt(0)).build();
        },
    );
}

fn create_pure(b: &mut Bencher) {
    b.iter_with_large_setup(
        World::new,
        |w| {
            w.entities().create();
        },
    );
}

fn delete_now(b: &mut Bencher) {
    b.iter_with_setup(
        || {
            let mut w = create_world();
            let eids: Vec<_> = (0..100).map(|_| w.create_entity().build()).collect();

            (w, eids)
        },
        |(mut w, mut eids)| {
            if let Some(id) = eids.pop() {
                w.delete_entity(id).unwrap()
            }
        },
    );
}

fn delete_now_with_storage(b: &mut Bencher) {
    b.iter_with_setup(
        || {
            let mut w = create_world();
            let eids: Vec<_> = (0..100)
                .map(|_| w.create_entity().with(CompInt(1)).build())
                .collect();

            (w, eids)
        },
        |(mut w, mut eids)| {
            if let Some(id) = eids.pop() {
                w.delete_entity(id).unwrap()
            }
        },
    );
}

fn delete_later(b: &mut Bencher) {
    let mut w = World::new();
    let mut eids: Vec<_> = (0..10_000_000).map(|_| w.create_entity().build()).collect();
    b.iter(|| {
        if let Some(id) = eids.pop() {
            w.entities().delete(id).unwrap()
        }
    });
}

fn create_after_delete(b: &mut Bencher) {
    use rand::prelude::*;

    let mut rng = thread_rng();
    b.iter_with_setup(
        || {
            let mut w = World::new();
            let eids: Vec<_> = (0..1000).map(|_| w.create_entity().build()).collect();

            (0..1000)
                .choose_multiple(&mut rng, 100)
                .into_iter()
                .map(|i| eids[i])
                .for_each(|e| {
                    w.delete_entity(e)
                        .expect("Failed deleting entity in 'create after delete' setup");
                });

            w.maintain();

            w
        },
        |mut w| {
            for _ in 0..100 {
                w.create_entity().build();
            }
        },
    )
}

fn create_after_delete_extreme(b: &mut Bencher) {
    b.iter_with_setup(
        || {
            let mut w = World::new();

            let eids: Vec<_> = (0..1000).map(|_| w.create_entity().build()).collect();

            w.delete_entity(eids[0])
                .expect("Failed deleting first entity in 'create after delete extreme' setup");
            w.delete_entity(eids[999])
                .expect("Failed deleting last entity in 'create after delete extreme' setup");
            w.maintain();

            w
        },
        |mut w| {
            w.create_entity().build();
            w.create_entity().build();
        },
    )
}

fn maintain_noop(b: &mut Bencher) {
    let mut w = World::new();
    b.iter(|| {
        w.maintain();
    });
}

fn maintain_add_later(b: &mut Bencher) {
    let mut w = World::new();
    b.iter(|| {
        w.entities().create();
        w.maintain();
    });
}

fn maintain_delete_later(b: &mut Bencher) {
    let mut w = World::new();
    let mut eids: Vec<_> = (0..10_000_000).map(|_| w.create_entity().build()).collect();
    b.iter(|| {
        if let Some(id) = eids.pop() {
            w.entities().delete(id).unwrap();
        }
        w.maintain();
    });
}

fn join_single_threaded(b: &mut Bencher) {
    use test::black_box;

    let mut world = World::new();
    world.register::<CompInt>();

    {
        let entities: Vec<_> = world.create_iter().take(50_000).collect();
        let mut comp_int = world.write_storage();
        for (i, e) in entities.iter().enumerate() {
            comp_int.insert(*e, CompInt(i as i32)).unwrap();
        }
    }

    b.iter(|| {
        for comp in world.read_storage::<CompInt>().join() {
            black_box(comp.0 * comp.0);
        }
    })
}

fn join_multi_threaded(b: &mut Bencher) {
    use rayon::prelude::*;
    use test::black_box;

    let mut world = World::new();
    world.register::<CompInt>();

    {
        let entities: Vec<_> = world.create_iter().take(50_000).collect();
        let mut comp_int = world.write_storage();
        for (i, e) in entities.iter().enumerate() {
            comp_int.insert(*e, CompInt(i as i32)).unwrap();
        }
    }

    b.iter(|| {
        world.read_storage::<CompInt>().par_join().for_each(|comp| {
            black_box(comp.0 * comp.0);
        })
    })
}

fn fetch_four_storages(b: &mut Bencher) {
    let world = create_world();
    b.iter(|| {
        (
            world.read_storage::<CompInt>(),
            world.read_storage::<CompIntTwo>(),
            world.read_storage::<CompIntThree>(),
            world.read_storage::<CompBool>(),
        )
    })
}

fn world_benchmarks(c: &mut Criterion) {
    c.bench_function("world build", world_build)
        .bench_function("create now", create_now)
        .bench_function("create pure", create_pure)
        .bench_function("create now with storage", create_now_with_storage)
        .bench_function("delete now", delete_now)
        .bench_function("delete now with storage", delete_now_with_storage)
        .bench_function("delete later", delete_later)
        .bench_function("create after delete", create_after_delete)
        .bench_function("create after delete extreme", create_after_delete_extreme)
        .bench_function("maintain noop", maintain_noop)
        .bench_function("maintain add later", maintain_add_later)
        .bench_function("maintain delete later", maintain_delete_later)
        .bench_function("join single threaded", join_single_threaded)
        .bench_function("join multi threaded", join_multi_threaded)
        .bench_function("fetch four storages", fetch_four_storages);
}

criterion_group!(world, world_benchmarks);

criterion_main!(world);
