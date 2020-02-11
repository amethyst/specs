use specs::{prelude::*, storage::HashMapStorage};

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
// Resources are unique and can be accessed
// from systems using the same sync strategy
// as component storages

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
        Self { counter: 0 }
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
        // This line can't fail because we just made the entity.
        data.comp_int.insert(entity, CompInt(self.counter)).unwrap();

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
        Self(None)
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

        for (entity, value) in (&data.entities, &data.comp_int).join() {
            if value.0 >= max {
                max = value.0;
                max_entity = Some(entity);
            }
        }

        self.0 = max_entity;
    }
}

#[cfg(feature = "parallel")]
struct JoinParallel;

#[cfg(feature = "parallel")]
impl<'a> System<'a> for JoinParallel {
    type SystemData = (
        ReadStorage<'a, CompBool>,
        ReadStorage<'a, CompInt>,
        WriteStorage<'a, CompFloat>,
    );

    fn run(&mut self, (comp_bool, comp_int, mut comp_float): Self::SystemData) {
        use specs::rayon::prelude::*;
        (&comp_bool, &comp_int, &mut comp_float)
            .par_join()
            // only iterate over entities with a `CompBool(true)`
            .filter(|&(b, _, _)| b.0)
            // set the `CompFloat` value to the float repr of `CompInt`
            .for_each(|(_, i, f)| f.0 += i.0 as f32);
    }
}

/// Takes every `CompFloat` and tries to add `CompInt` if it exists.
struct AddIntToFloat;

impl<'a> System<'a> for AddIntToFloat {
    type SystemData = (ReadStorage<'a, CompInt>, ReadStorage<'a, CompFloat>);

    fn run(&mut self, (comp_int, comp_float): Self::SystemData) {
        // This system demonstrates the use of `.maybe()`.
        // As the name implies, it doesn't filter any entities; it yields an
        // `Option<CompInt>`. So the `join` will yield all entities that have a
        // `CompFloat`, just returning a `CompInt` if the entity happens to have
        // one.
        for (f, i) in (&comp_float, comp_int.maybe()).join() {
            let sum = f.0 + i.map(|i| i.0 as f32).unwrap_or(0.0);
            println!("Result: sum = {}", sum);
        }

        // An alternative way to write this out:
        // (note that `entities` is just another system data of type
        // `Entities<'a>`)
        //
        // ```
        // for (entity, f) in (&entities, &comp_float).join() {
        //     let i = comp_int.get(e); // retrieves the component for the current entity
        //
        //     let sum = f.0 + i.map(|i| i.0 as f32).unwrap_or(0.0);
        //     println!("Result: sum = {}", sum);
        // }
        // ```
    }
}

fn main() {
    let mut w = World::new();
    // This builds our dispatcher, which contains the systems.
    // Every system has a name and can depend on other systems.
    // "check_positive" depends on  "print_bool" for example,
    // because we want to print the components before executing
    // `SysCheckPositive`.
    let mut dispatcher_builder = DispatcherBuilder::new()
        .with(SysPrintBool, "print_bool", &[])
        .with(SysCheckPositive, "check_positive", &["print_bool"])
        .with(SysStoreMax::new(), "store_max", &["check_positive"])
        .with(SysSpawn::new(), "spawn", &[])
        .with(SysPrintBool, "print_bool2", &["check_positive"]);

    #[cfg(feature = "parallel")]
    {
        dispatcher_builder = dispatcher_builder.with(JoinParallel, "join_par", &[])
    }

    let mut dispatcher = dispatcher_builder
        .with_barrier() // we want to make sure all systems finished before running the last one
        .with(AddIntToFloat, "add_float_int", &[])
        .build();

    // setup() will setup all Systems we added, registering all resources and
    // components that they will try to read from and write to
    dispatcher.setup(&mut w);

    // create_entity() of World provides with an EntityBuilder to add components to
    // an Entity
    w.create_entity()
        .with(CompInt(4))
        .with(CompBool(false))
        .build();
    // build() returns an entity, we will use it later to perform a deletion
    let e = w
        .create_entity()
        .with(CompInt(9))
        .with(CompBool(true))
        .build();
    w.create_entity()
        .with(CompInt(-1))
        .with(CompBool(false))
        .build();
    w.create_entity().with(CompInt(127)).build();
    w.create_entity().with(CompBool(false)).build();
    w.create_entity().with(CompFloat(0.1)).build();

    dispatcher.dispatch(&w);
    w.maintain();

    // Insert a component, associated with `e`.
    if let Err(err) = w.write_storage().insert(e, CompFloat(4.0)) {
        eprintln!("Failed to insert component! {:?}", err);
    }

    dispatcher.dispatch(&w);
    w.maintain();
}
