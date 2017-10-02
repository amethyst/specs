extern crate rayon;
extern crate shred;
#[macro_use]
extern crate shred_derive;
extern crate specs;

use specs::{Component, DenseVecStorage, DispatcherBuilder, Entities, Entity, HashMapStorage, Join,
            ParJoin, ReadStorage, System, VecStorage, World, WriteStorage};

// -- Components --
// A component exists for 0..n
// entities.

#[derive(Clone, Debug)]
struct CompInt(i32);

impl Component for CompInt {
    // Storage is used to store all data for components of this type
    // VecStorage is meant to be used for components that are in almost every entity
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug)]
struct CompBool(bool);

impl Component for CompBool {
    // HashMapStorage is better for components that are met rarely
    type Storage = HashMapStorage<Self>;
}

#[derive(Clone, Debug)]
struct CompFloat(f32);

impl Component for CompFloat {
    type Storage = DenseVecStorage<Self>;
}

// -- Resources --
// Resources can be accessed
// from systems.

#[derive(Clone, Debug)]
struct Sum(usize);

// -- System Data --
// Each system has an associated
// data type.

#[derive(SystemData)]
struct IntAndBoolData<'a> {
    comp_int: ReadStorage<'a, CompInt>,
    comp_bool: WriteStorage<'a, CompBool>,
}

#[derive(SystemData)]
struct SpawnData<'a> {
    comp_int: WriteStorage<'a, CompInt>,
    entities: Entities<'a>,
}

#[derive(SystemData)]
struct StoreMaxData<'a> {
    comp_float: ReadStorage<'a, CompFloat>,
    comp_int: ReadStorage<'a, CompInt>,
    entities: Entities<'a>,
}

// -- Systems --

struct SysPrintBool;

impl<'a> System<'a> for SysPrintBool {
    type SystemData = ReadStorage<'a, CompBool>;

    fn run(&mut self, data: ReadStorage<CompBool>) {
        for b in (&data).join() {
            println!("Bool: {:?}", b);
        }
    }
}

struct SysCheckPositive;

impl<'a> System<'a> for SysCheckPositive {
    type SystemData = IntAndBoolData<'a>;

    fn run(&mut self, mut data: IntAndBoolData) {
        // Join merges the two component storages,
        // so you get all (CompInt, CompBool) pairs.
        for (ci, cb) in (&data.comp_int, &mut data.comp_bool).join() {
            cb.0 = ci.0 > 0;
        }
    }
}

struct SysSpawn {
    counter: i32,
}

impl SysSpawn {
    fn new() -> Self {
        SysSpawn { counter: 0 }
    }
}

impl<'a> System<'a> for SysSpawn {
    type SystemData = SpawnData<'a>;

    fn run(&mut self, mut data: SpawnData) {
        if self.counter == 0 {
            let entity = data.entities.join().next().unwrap();
            let _ = data.entities.delete(entity);
        }

        let entity = data.entities.create();
        data.comp_int.insert(entity, CompInt(self.counter));

        self.counter += 1;

        if self.counter > 100 {
            self.counter = 0;
        }
    }
}

/// Stores the entity with
/// the greatest int.
struct SysStoreMax(Option<Entity>);

impl SysStoreMax {
    fn new() -> Self {
        SysStoreMax(None)
    }
}

impl<'a> System<'a> for SysStoreMax {
    type SystemData = StoreMaxData<'a>;

    fn run(&mut self, data: StoreMaxData) {
        use std::i32::MIN;

        // Let's print information about
        // last run's entity
        if let Some(e) = self.0 {
            if let Some(f) = data.comp_float.get(e) {
                println!("Entity with biggest int has float value {:?}", f);
            } else {
                println!("Entity with biggest int has no float value");
            }
        }

        let mut max_entity = None;
        let mut max = MIN;

        for (entity, value) in (&*data.entities, &data.comp_int).join() {
            if value.0 >= max {
                max = value.0;
                max_entity = Some(entity);
            }
        }

        self.0 = max_entity;
    }
}

struct JoinParallel;

impl<'a> System<'a> for JoinParallel {
    type SystemData = (ReadStorage<'a, CompInt>, WriteStorage<'a, CompFloat>);

    fn run(&mut self, (comp_int, mut comp_float): Self::SystemData) {
        use rayon::prelude::*;
        (&comp_int, &mut comp_float)
            .par_join()
            .for_each(|(i, f)| f.0 += i.0 as f32);
    }
}

fn main() {
    let mut w = World::new();
    // All components types should be registered before working with them
    w.register::<CompInt>();
    w.register::<CompBool>();
    w.register::<CompFloat>();
    // create_entity() of World provides with an EntityBuilder to add components to an Entity
    w.create_entity()
        .with(CompInt(4))
        .with(CompBool(false))
        .build();
    // build() returns an entity, we will use it later to perform a deletion
    let e = w.create_entity()
        .with(CompInt(9))
        .with(CompBool(true))
        .build();
    w.create_entity()
        .with(CompInt(-1))
        .with(CompBool(false))
        .build();
    w.create_entity().with(CompInt(127)).build();
    w.create_entity().with(CompBool(false)).build();

    // resources can be installed, these are nothing fancy, but allow you
    // to pass data to systems and follow the same sync strategy as the
    // component storage does.
    w.add_resource(Sum(0xdead_beef));

    // This builds our dispatcher, which contains the systems.
    // Every system has a name and can depend on other systems.
    // "check_positive" depends on  "print_bool" for example,
    // because we want to print the components before executing
    // `SysCheckPositive`.
    let mut dispatcher = DispatcherBuilder::new()
        .add(SysPrintBool, "print_bool", &[])
        .add(SysCheckPositive, "check_positive", &["print_bool"])
        .add(SysStoreMax::new(), "store_max", &["check_positive"])
        .add(SysSpawn::new(), "spawn", &[])
        .add(SysPrintBool, "print_bool2", &["check_positive"])
        .add(JoinParallel, "join_par", &[])
        .build();

    dispatcher.dispatch(&w.res);
    w.maintain();

    // Insert a component, associated with `e`.
    w.write().insert(e, CompFloat(4.0));

    dispatcher.dispatch(&w.res);
    w.maintain();
}
