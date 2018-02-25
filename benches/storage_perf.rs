use criterion::{Bencher, Criterion};
use specs::prelude::*;

#[derive(Clone, Debug)]
pub struct GridVelocity {
    pub dx: i16,
    pub dy: i16,
}
impl GridVelocity {
    pub fn new() -> Self {
        GridVelocity { dx: 0, dy: 0 }
    }
}
impl Component for GridVelocity {
    type Storage = VecStorage<Self>;
}

fn bench_vec(b: &mut Bencher) {
    let mut vels: Vec<GridVelocity> = (0..1000 * 1000).map(|_i| GridVelocity::new()).collect();
    b.iter(|| {
        vels.iter_mut().for_each(|vel| {
            vel.dx += 1;
        });
    });
}

pub fn bench_vecstorage(b: &mut Bencher) {
    use specs::storage::UnprotectedStorage;
    use std::default::Default;

    let mut vel_storage: VecStorage<GridVelocity> = Default::default();

    for i in 0..1000 * 1000 {
        unsafe {
            vel_storage.insert(i, GridVelocity::new());
        }
    }

    b.iter(|| {
        for i in 0..1000 * 1000 {
            let mut vel = unsafe { vel_storage.get_mut(i) };
            vel.dx += 1;
        }
    });
}

pub fn bench_storage_entity_prefetch(b: &mut Bencher) {
    let mut world = World::new();
    world.register::<GridVelocity>();
    for _i in 0..1000 * 1000 {
        world.create_entity().with(GridVelocity::new()).build();
    }

    let mut vel_storage = world.write::<GridVelocity>();
    let ents = world.entities();
    let entities: Vec<Entity> = (0..1000 * 1000).map(|i| ents.entity(i)).collect();

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
}

criterion_group!(benches_storage_perfs, raw_vect_perf);
