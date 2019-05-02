#![feature(test)]

extern crate cgmath;
extern crate rand;
extern crate shred;
extern crate specs;
extern crate test;

use cgmath::Vector3;
use rand::thread_rng;
use shred::RunningTime;
use specs::prelude::*;
use specs::storage::{HashMapStorage, NullStorage};
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
    type SystemData = (
        ReadStorage<'a, Small>,
        WriteStorage<'a, Small2>,
    );

    fn run(&mut self, (small, mut small2): Self::SystemData) {
        let mut c = 0;
        for (s, mut s2) in (&small, &mut small2).join() {
            c += 1;
        }
        println!("c: {}", c);
    }
}

struct BigSystem;

impl<'a> System<'a> for BigSystem {
    type SystemData = (
        WriteStorage<'a, Big>,
    );

    fn run(&mut self, (mut big,): Self::SystemData) {
        let mut c = 0;
        for (mut s) in (&mut big,).join() {
            c += 1;
        }
        println!("c: {}", c);
    }
}

#[bench]
fn bench_big(b: &mut Bencher) {
    let mut w = World::new();

    w.register::<Big>();

    for x in 0..100000 {
        w.create_entity()
            .with(Big(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0)))
            .build();
    }

    let mut d = DispatcherBuilder::new()
        .with(BigSystem, "big_sys", &[])
        .build();

    b.iter(|| {
        d.dispatch(&mut w.res);
        w.maintain();
    })
}

#[bench]
fn bench_small(b: &mut Bencher) {
    let mut w = World::new();

    w.register::<Small>();
    w.register::<Small2>();

    for x in 0..100000 {
        w.create_entity()
            .with(Small(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0)))
            .with(Small2(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0)))
            .build();
    }

    let mut d = DispatcherBuilder::new()
        .with(SmallSystem, "small_sys", &[])
        .build();

    b.iter(|| {
        d.dispatch(&mut w.res);
        w.maintain();
    })
}
