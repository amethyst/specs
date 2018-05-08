#![feature(test)]

#[macro_use]
extern crate criterion;
extern crate specs;
extern crate test;

macro_rules! group {
    ($name:ident,$($benches:path),*) => {
        pub fn $name(c: &mut Criterion) {
            $(
                $benches(c);
            )*
        }
    };
}

mod storage_cmp;
mod storage_sparse;
mod storage_perf;

pub use test::black_box;

use storage_cmp::benches_storages;
use storage_sparse::benches_sparse;
use storage_perf::benches_storage_perfs;

criterion_main!(benches_storages, benches_sparse, benches_storage_perfs);
