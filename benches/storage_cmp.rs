use criterion::{Bencher, Criterion};
use specs::{prelude::*, storage};

use super::black_box;

fn storage_insert<C>(b: &mut Bencher, num: usize)
where
    C: Component + Default,
    C::Storage: Default,
{
    b.iter_with_setup(
        || {
            let mut world = World::new();

            world.register::<C>();

            world
        },
        |world| {
            let entities = world.entities();
            let mut storage = world.write_storage::<C>();

            for e in entities.create_iter().take(num) {
                storage.insert(e, C::default()).unwrap();
            }
        },
    )
}

fn storage_remove<C>(b: &mut Bencher, num: usize)
where
    C: Component + Default,
    C::Storage: Default,
{
    b.iter_with_setup(
        || {
            let mut world = World::new();

            world.register::<C>();

            {
                let entities = world.entities();
                let mut storage = world.write_storage::<C>();

                for e in entities.create_iter().take(num) {
                    storage.insert(e, C::default()).unwrap();
                }
            }

            world
        },
        |world| {
            let entities = world.entities();
            let mut storage = world.write_storage::<C>();

            for e in entities.join() {
                storage.remove(e);
            }
        },
    )
}

fn storage_get<C>(b: &mut Bencher, num: usize)
where
    C: Component + Default,
    C::Storage: Default,
{
    b.iter_with_setup(
        || {
            let mut world = World::new();

            world.register::<C>();

            {
                let entities = world.entities();
                let mut storage = world.write_storage::<C>();

                for e in entities.create_iter().take(num) {
                    storage.insert(e, C::default()).unwrap();
                }
            }

            world
        },
        |world| {
            let entities = world.entities();
            let storage = world.read_storage::<C>();

            for e in entities.join() {
                black_box(storage.get(e));
            }
        },
    )
}

macro_rules! decl_comp {
    ($bytes:expr, $store:ident) => {
        #[derive(Default)]
        struct Comp {
            _x: [u8; $bytes],
        }

        impl Component for Comp {
            type Storage = storage::$store<Self>;
        }
    };
}

macro_rules! insert {
    ($b:ident, $num:expr, $bytes:expr, $store:ident) => {{
        decl_comp!($bytes, $store);

        storage_insert::<Comp>($b, $num)
    }};
}

macro_rules! remove {
    ($b:ident, $num:expr, $bytes:expr, $store:ident) => {{
        decl_comp!($bytes, $store);

        storage_remove::<Comp>($b, $num)
    }};
}

macro_rules! get {
    ($b:ident, $num:expr, $bytes:expr, $store:ident) => {{
        decl_comp!($bytes, $store);

        storage_get::<Comp>($b, $num)
    }};
}

#[rustfmt::skip]
fn insert_benches(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "insert 1b/dense",
        |b, &&i| insert!(b, i, 1, DenseVecStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "insert 1b/btree",
        |b, &&i| insert!(b, i, 1, BTreeStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "insert 1b/hash",
        |b, &&i| insert!(b, i, 1, HashMapStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "insert 1b/vec",
        |b, &&i| insert!(b, i, 1, VecStorage),
        &[1, 16, 64, 256, 1024],
    );

    c.bench_function_over_inputs(
        "insert 32b/dense",
        |b, &&i| insert!(b, i, 32, DenseVecStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "insert 32b/btree",
        |b, &&i| insert!(b, i, 32, BTreeStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "insert 32b/hash",
        |b, &&i| insert!(b, i, 32, HashMapStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "insert 32b/vec",
        |b, &&i| insert!(b, i, 32, VecStorage),
        &[1, 16, 64, 256, 1024],
    );
}

#[rustfmt::skip]
fn remove_benches(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "remove 1b/dense",
        |b, &&i| remove!(b, i, 1, DenseVecStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "remove 1b/btree",
        |b, &&i| remove!(b, i, 1, BTreeStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "remove 1b/hash",
        |b, &&i| remove!(b, i, 1, HashMapStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "remove 1b/vec",
        |b, &&i| remove!(b, i, 1, VecStorage),
        &[1, 16, 64, 256, 1024],
    );

    c.bench_function_over_inputs(
        "remove 32b/dense",
        |b, &&i| remove!(b, i, 32, DenseVecStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "remove 32b/btree",
        |b, &&i| remove!(b, i, 32, BTreeStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "remove 32b/hash",
        |b, &&i| remove!(b, i, 32, HashMapStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "remove 32b/vec",
        |b, &&i| remove!(b, i, 32, VecStorage),
        &[1, 16, 64, 256, 1024],
    );
}

#[rustfmt::skip]
fn get_benches(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "get 1b/dense",
        |b, &&i| remove!(b, i, 1, DenseVecStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "get 1b/btree",
        |b, &&i| remove!(b, i, 1, BTreeStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "get 1b/hash",
        |b, &&i| remove!(b, i, 1, HashMapStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "get 1b/vec",
        |b, &&i| remove!(b, i, 1, VecStorage),
        &[1, 16, 64, 256, 1024],
    );

    c.bench_function_over_inputs(
        "get 32b/dense",
        |b, &&i| get!(b, i, 32, DenseVecStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "get 32b/btree",
        |b, &&i| get!(b, i, 32, BTreeStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "get 32b/hash",
        |b, &&i| get!(b, i, 32, HashMapStorage),
        &[1, 16, 64, 256, 1024],
    ).bench_function_over_inputs(
        "get 32b/vec",
        |b, &&i| get!(b, i, 32, VecStorage),
        &[1, 16, 64, 256, 1024],
    );
}

criterion_group!(
    benches_storages,
    insert_benches,
    remove_benches,
    get_benches
);
