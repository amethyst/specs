#![feature(test)]

extern crate specs;
extern crate test;

macro_rules! setup {
    ($num:expr => [ $( $comp:ty ),* ] ) => {
        pub fn setup(insert: bool, sparsity: u32) -> (World, Vec<Entity>) {
            let mut w = World::new();
            $(
                w.register::<$comp>();
            )*

            let eids: Vec<_> = (0..$num)
                .map(|i| {
                    let mut builder = w.create_entity();
                    if insert && i % sparsity == 0 {
                        $(
                            builder = builder.with::<$comp>(<$comp>::default());
                        )*
                    }
                    builder.build()
                })
                .collect();

            (w, eids)
        }
    }
}

macro_rules! gap {
    ( $name:ident => $sparsity:expr ) => {
        mod $name {
            use super::{CompInt, CompBool, setup};
            use test::{Bencher, black_box};

            #[bench]
            fn insert(bencher: &mut Bencher) {
                let (world, entities) = setup(false, $sparsity);
                let mut ints = world.write::<CompInt>();
                let mut bools = world.write::<CompBool>();

                bencher.iter(move || {
                    for &entity in &entities {
                        ints.insert(entity, CompInt::default());
                        bools.insert(entity, CompBool::default());
                    }
                });
            }
            #[bench]
            fn remove(bencher: &mut Bencher) {
                let (world, entities) = setup(true, $sparsity);
                let mut ints = world.write::<CompInt>();
                let mut bools = world.write::<CompBool>();

                bencher.iter(move || {
                    for &entity in &entities {
                        ints.remove(entity);
                        bools.remove(entity);
                    }
                });
            }
            #[bench]
            fn get(bencher: &mut Bencher) {
                let (world, entities) = setup(true, $sparsity);
                let ints = world.read::<CompInt>();
                let bools = world.read::<CompBool>();

                bencher.iter(move || {
                    for &entity in &entities {
                        black_box(ints.get(entity));
                        black_box(bools.get(entity));
                    }
                });
            }
        }
    }
}

macro_rules! tests {
    ($mod:ident => $storage:ty) => {
        mod $mod {
            use specs::{Component, Entity, World};

            pub static NUM: u32 = 100_000;

            pub struct CompInt(u32);
            pub struct CompBool(bool);

            impl Default for CompInt {
                fn default() -> Self {
                    CompInt(0)
                }
            }

            impl Default for CompBool {
                fn default() -> Self {
                    CompBool(true)
                }
            }

            impl Component for CompInt {
                type Storage = $storage;
            }
            impl Component for CompBool {
                type Storage = $storage;
            }

            setup!(NUM => [ CompInt, CompBool ]);

            gap!(sparse_1 => 1);
            gap!(sparse_2 => 2);
            gap!(sparse_4 => 4);
            gap!(sparse_8 => 8);
            gap!(sparse_128 => 128);
            gap!(sparse_256 => 256);
            gap!(sparse_512 => 512);
            gap!(sparse_1024 => 1024);
            gap!(sparse_10000 => 10_000);
            gap!(sparse_50000 => 50_000);
        }
    }
}

tests!(vec_storage => ::specs::VecStorage<Self>);
tests!(dense_vec_storage => ::specs::DenseVecStorage<Self>);
tests!(hashmap_storage => ::specs::HashMapStorage<Self>);
tests!(btree_storage => ::specs::BTreeStorage<Self>);
#[cfg(feature = "rudy")]
tests!(rudy_storage => ::specs::RudyStorage<Self>);
