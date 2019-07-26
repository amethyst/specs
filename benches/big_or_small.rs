#![feature(test)]

extern crate nalgebra;
extern crate rand;
extern crate shred;
extern crate specs;
extern crate test;

use nalgebra::Vector3;
use specs::prelude::*;
use test::Bencher;

type Vec3 = Vector3<f32>;

// -- Components --
#[derive(Clone, Debug)]
struct Small(Vec3, Vec3);

impl Component for Small {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug)]
struct Small2(Vec3, Vec3);

impl Component for Small2 {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug)]
struct Big(Vec3, Vec3, Vec3, Vec3);

impl Component for Big {
    type Storage = VecStorage<Self>;
}

// -- Systems --

struct SmallSystem;

impl<'a> System<'a> for SmallSystem {
    type SystemData = (ReadStorage<'a, Small>, WriteStorage<'a, Small2>);

    fn run(&mut self, (small, mut small2): Self::SystemData) {
        for (s, s2) in (&small, &mut small2).join() {
            s2.0.y += s.0.x;
        }
    }
}

struct BigSystem;

impl<'a> System<'a> for BigSystem {
    type SystemData = (WriteStorage<'a, Big>,);

    fn run(&mut self, (mut big,): Self::SystemData) {
        for (b,) in (&mut big,).join() {
            b.0.y += b.0.x;
        }
    }
}

#[bench]
fn bench_big(b: &mut Bencher) {
    let mut world = World::new();

    world.register::<Big>();

    for _ in 0..100000 {
        world
            .create_entity()
            .with(Big(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 0.0),
            ))
            .build();
    }

    let mut dispatch = DispatcherBuilder::new()
        .with(BigSystem, "big_sys", &[])
        .build();

    b.iter(|| {
        dispatch.dispatch(&mut world);
        world.maintain();
    })
}

#[bench]
fn bench_small(b: &mut Bencher) {
    let mut world = World::new();

    world.register::<Small>();
    world.register::<Small2>();

    for _ in 0..100000 {
        world
            .create_entity()
            .with(Small(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0)))
            .with(Small2(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0)))
            .build();
    }

    let mut dispatch = DispatcherBuilder::new()
        .with(SmallSystem, "small_sys", &[])
        .build();

    b.iter(|| {
        dispatch.dispatch(&mut world);
        world.maintain();
    })
}
