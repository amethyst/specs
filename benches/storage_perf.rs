use criterion::{Bencher, Criterion};
use specs::prelude::*;

#[derive(Clone, Debug)]
pub struct GridVelocity {
    pub dx: i16,
    pub dy: i16,
}
impl GridVelocity {
    pub fn new() -> Self { GridVelocity { dx:0, dy:0 } }
}
impl Component for GridVelocity {
    type Storage = VecStorage<Self>;
}

fn bench_vec(b: &mut Bencher) {
    let mut vels: Vec<GridVelocity> = (0..1000*1000).map(|_i| { GridVelocity::new() }).collect();
    b.iter(|| {
        vels.iter_mut().for_each(|vel| {
        // for vel in  {
            vel.dx += 1;
        });
    });
} 

pub fn bench_vecstorage(b: &mut Bencher) {
    use std::default::Default;
    use specs::storage::UnprotectedStorage;

    let mut vel_storage: VecStorage<GridVelocity> = Default::default();

    for i in 0..1000*1000 {
        unsafe { vel_storage.insert(i, GridVelocity::new()); }
    }

    b.iter(|| {
        for i in 0..1000*1000 {
            let mut vel = unsafe { vel_storage.get_mut(i) };
            vel.dx += 1;
        }
    });
}  

pub fn bench_storage_entity_prefetch(b: &mut Bencher) {
    let mut world = World::new();
    world.register::<GridVelocity>();
    for _i in 0..1000*1000 {
         world.create_entity().with(GridVelocity::new()).build();
    }

    let mut vel_storage = world.write::<GridVelocity>();
    let ents = world.entities();
    let entities: Vec<Entity> = (0..1000*1000).map(|i| ents.entity(i)).collect();

    b.iter(|| {
        for e in entities.iter() {
            let mut vel = vel_storage.get_mut(*e).unwrap();
            vel.dx += 1;
        }
    });
}  

#[cfg_attr(rustfmt, rustfmt_skip)]
fn raw_vect_perf(c: &mut Criterion) {
    c.bench_function("Vec iteration", bench_vec);
    c.bench_function("VecStorage iteration", bench_vecstorage);
    c.bench_function("WriteStorage iteration", bench_storage_entity_prefetch);
    // c.bench_function_over_inputs(
    //     "get 32b/dense",
    //     |b, &&i| get!(b, i, 32, DenseVecStorage),
    //     &[1, 16, 64, 256, 1024],
    // ).bench_function_over_inputs(
    //     "get 32b/btree",
    //     |b, &&i| get!(b, i, 32, BTreeStorage),
    //     &[1, 16, 64, 256, 1024],
    // ).bench_function_over_inputs(
    //     "get 32b/hash",
    //     |b, &&i| get!(b, i, 32, HashMapStorage),
    //     &[1, 16, 64, 256, 1024],
    // ).bench_function_over_inputs(
    //     "get 32b/vec",
    //     |b, &&i| get!(b, i, 32, VecStorage),
    //     &[1, 16, 64, 256, 1024],
    // );
}

criterion_group!(benches_storage_perfs, raw_vect_perf);
