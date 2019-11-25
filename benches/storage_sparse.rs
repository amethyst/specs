use criterion::{Bencher, Criterion};

use super::black_box;

macro_rules! setup {
    ($num:expr => [ $( $comp:ty ),* ] ) => {
        pub fn setup(filter: bool, insert: bool, sparsity: u32) -> (World, Vec<Entity>) {
            let mut w = World::new();
            $(
                w.register::<$comp>();
            )*

            let eids: Vec<_> = (0..$num)
                .flat_map(|i| {
                    let mut builder = w.create_entity();
                    if insert {
                        if i % sparsity == 0 {
                        $(
                            builder = builder.with::<$comp>(<$comp>::default());
                        )*
                        }
                    }

                    if !filter || i % sparsity == 0 {
                        Some(builder.build())
                    } else {
                        None
                    }
                })
                .collect();

            (w, eids)
        }
    }
}

macro_rules! gap {
    ($storage:ident, $name:ident => $sparsity:expr) => {
        mod $name {
            use super::{
                super::{black_box, Bencher, Criterion},
                setup, CompBool, CompInt,
            };
            use specs::prelude::*;

            fn insert(bencher: &mut Bencher) {
                let (world, entities) = setup(true, false, $sparsity);
                let mut ints = world.write_storage::<CompInt>();
                let mut bools = world.write_storage::<CompBool>();

                bencher.iter(move || {
                    for &entity in &entities {
                        ints.insert(entity, CompInt::default()).unwrap();
                        bools.insert(entity, CompBool::default()).unwrap();
                    }
                });
            }

            fn remove(bencher: &mut Bencher) {
                let (world, entities) = setup(true, true, $sparsity);
                let mut ints = world.write_storage::<CompInt>();
                let mut bools = world.write_storage::<CompBool>();

                bencher.iter(move || {
                    for &entity in &entities {
                        ints.remove(entity);
                        bools.remove(entity);
                    }
                });
            }

            fn get(bencher: &mut Bencher) {
                let (world, entities) = setup(false, true, $sparsity);
                let ints = world.read_storage::<CompInt>();
                let bools = world.read_storage::<CompBool>();

                bencher.iter(move || {
                    for &entity in &entities {
                        black_box(ints.get(entity));
                        black_box(bools.get(entity));
                    }
                });
            }

            pub fn benches(c: &mut Criterion) {
                c.bench_function(
                    &format!("sparse insert {}/{}", $sparsity, stringify!($storage)),
                    |b| insert(b),
                )
                .bench_function(
                    &format!("sparse remove {}/{}", $sparsity, stringify!($storage)),
                    |b| remove(b),
                )
                .bench_function(
                    &format!("sparse get {}/{}", $sparsity, stringify!($storage)),
                    |b| get(b),
                );
            }
        }
    };
}

macro_rules! tests {
    ($mod:ident => $storage:ident) => {
        mod $mod {
            use criterion::Criterion;
            use specs::prelude::*;

            pub static NUM: u32 = 100_000;

            pub struct CompInt(u32);
            pub struct CompBool(bool);

            impl Default for CompInt {
                fn default() -> Self {
                    Self(0)
                }
            }

            impl Default for CompBool {
                fn default() -> Self {
                    Self(true)
                }
            }

            impl Component for CompInt {
                type Storage = ::specs::storage::$storage<Self>;
            }
            impl Component for CompBool {
                type Storage = ::specs::storage::$storage<Self>;
            }

            setup!(NUM => [ CompInt, CompBool ]);

            gap!($storage, sparse_1 => 1);
            gap!($storage, sparse_2 => 2);
            gap!($storage, sparse_4 => 4);
            gap!($storage, sparse_8 => 8);
            gap!($storage, sparse_128 => 128);
            gap!($storage, sparse_256 => 256);
            gap!($storage, sparse_512 => 512);
            gap!($storage, sparse_1024 => 1024);
            gap!($storage, sparse_10000 => 10_000);
            gap!($storage, sparse_50000 => 50_000);

            group!(
                benches,
                sparse_1::benches,
                sparse_2::benches,
                sparse_4::benches,
                sparse_8::benches,
                sparse_128::benches,
                sparse_256::benches,
                sparse_512::benches,
                sparse_1024::benches,
                sparse_10000::benches,
                sparse_50000::benches
            );
        }
    };
}

tests!(vec_storage => VecStorage);
tests!(dense_vec_storage => DenseVecStorage);
tests!(hashmap_storage => HashMapStorage);
tests!(btree_storage => BTreeStorage);

criterion_group!(
    benches_sparse,
    vec_storage::benches,
    dense_vec_storage::benches,
    hashmap_storage::benches,
    btree_storage::benches
);
