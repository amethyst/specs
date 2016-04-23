extern crate specs;

#[derive(Clone, Debug)]
struct CompInt(i8);
impl specs::Component for CompInt {
    // Storage is used to store all data for components of this type
    // VecStorage is meant to be used for components that are in almost every entity
    type Storage = specs::VecStorage<CompInt>;
}

#[derive(Clone, Debug)]
struct CompBool(bool);
impl specs::Component for CompBool {
    // HashMapStorage is better for componets that are met rarely
    type Storage = specs::HashMapStorage<CompBool>;
}

#[derive(Clone, Debug)]
struct CompFloat(f32);
impl specs::Component for CompFloat {
    type Storage = specs::VecStorage<CompFloat>;
}

fn main() {
    let (e, mut planner) = {
        let mut w = specs::World::new();
        // All components types should be registered before working with them
        w.register::<CompInt>();
        w.register::<CompBool>();
        w.register::<CompFloat>();
        // create_now() of World provides with an EntityBuilder to add components to an Entity
        w.create_now().with(CompInt(4)).with(CompBool(false)).build();
        // build() returns an entity, we will use it later to perform a deletion
        let e = w.create_now().with(CompInt(9)).with(CompBool(true)).build();
        w.create_now().with(CompInt(-1)).with(CompBool(false)).build();
        w.create_now().with(CompInt(127)).build();
        w.create_now().with(CompBool(false)).build();
        // Planner is used to run systems on the specified world with a specified number of threads
        (e, specs::Planner::<()>::new(w, 4))
    };

    // Planner only runs closure on entites with specified components, for example:
    // We have 5 entities and this will print only 4 of them
    println!("Only entities with CompBool present:");
    planner.run0w1r(|b: &CompBool| {
        println!("Entity {}", b.0);
    });
    // wait waits for all scheduled systems to finish
    // If wait is not called, all systems are run in parallel, waiting on locks
    planner.wait();

    planner.run1w1r(|b: &mut CompBool, a: &CompInt| {
        b.0 = a.0 > 0;
    });
    // Deletes an entity instantly
    planner.world.delete_now(e);

    // Instead of using macros you can use run_custom() to build a system precisely
    planner.run_custom(|arg| {
        // fetch() borrows a world, so a system could lock neccessary storages
        // Can be called only once
        let (mut sa, sb) = arg.fetch(|w| {
            (w.write::<CompInt>(), w.read::<CompBool>())
        });

        // Instead of using the `entities` array you can
        // use the `Join` trait that is an optimized way of
        // doing the `get/get_mut` across entities.
        for (a, b) in specs::Join::from((&mut sa, &sb)) {
            a.0 += if b.0 {2} else {0};
        }

        // Dynamically creating and deleting entities
        let e0 = arg.create();
        sa.insert(e0, CompInt(-4));
        let e1 = arg.create();
        sa.insert(e1, CompInt(-5));
        arg.delete(e0);
    });
    println!("Only entities with CompInt and CompBool present:");
    planner.run0w2r(|a: &CompInt, b: &CompBool| {
        println!("Entity {} {}", a.0, b.0);
    });
    planner.run_custom(|arg| {
        use specs::{Storage, Join};
        let (mut sa, sb, entities) = arg.fetch(|w| {
            (w.write::<CompFloat>(), w.read::<CompInt>(), w.entities())
        });

        // Insert a component for each entity in sb
        for (eid, sb) in (&entities, &sb).join() {
            sa.insert(eid, CompFloat(sb.0 as f32));
        }

        for (eid, sa, sb) in (&entities, &mut sa, &sb).join() {
            assert_eq!(sa.0 as u32, sb.0 as u32);
            println!("eid[{:?}] = {:?} {:?}", eid, sa, sb);
        }
    });

    planner.wait();
}
