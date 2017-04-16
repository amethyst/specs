
extern crate specs;
#[cfg(feature="serialize")]
extern crate serde;
#[cfg(feature="serialize")]
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::fmt;

#[cfg(feature="serialize")]
mod s {
    use serde::{self, Serialize};
    use serde_json;
    use specs::{self, Join, PackedData, Gate};

    #[derive(Debug, Serialize, Deserialize)]
    struct CompSerialize {
        field: u32,
        other: bool,
    }
    impl specs::Component for CompSerialize {
        type Storage = specs::VecStorage<CompSerialize>;
    }

    struct SerializeSystem;
    impl specs::System<()> for SerializeSystem {
        fn run(&mut self, arg: specs::RunArg, _: ()) {
            use fmt::Display;

            let (entities, mut components) = arg.fetch(|w| {
                let entities = w.entities();
                let mut components = w.write::<CompSerialize>();

                (entities, components)
            });

            // Serialize the storage into JSON
            let mut buffer: Vec<u8> = Vec::new();
            let mut serializer = serde_json::Serializer::pretty(buffer);
            let result = components.serialize(&mut serializer);
            let serialized = serializer.into_inner().iter().map(|b| *b as char).collect::<String>(); 
            println!("Serialized storage: {}", serialized);

            // Get a list of all entities in the world
            let mut entity_list = Vec::new();
            for entity in (&entities).join() {
                entity_list.push(entity);
            }

            // Remove all components
            for (entity, _) in (&entities, &components.check()).join() {
                components.remove(entity);
            }

            // Deserialize with entity list
            let mut list = serde_json::from_str::<PackedData<CompSerialize>>(&serialized);
            println!("list: {:?}", list);
            let created = components.merge(&mut entity_list, list.unwrap());

            for (entity, _) in (&entities, &components.check()).join() {
                println!("Has: {:?}", entity);
            }
        }
    }

    pub fn main_redirect() {
        let mut world = specs::World::<()>::new();
        world.register::<CompSerialize>();

        world.create_pure();
        world.create_pure();
        world.create_now().with(CompSerialize { field: 5, other: true }).build();
        world.create_pure();
        world.create_pure();
        world.create_now().with(CompSerialize { field: 5, other: true }).build();
        world.create_now().with(CompSerialize { field: 10, other: false }).build();
        world.create_pure();
        world.create_now().with(CompSerialize { field: 0, other: false }).build();

        let mut planner = specs::Planner::<()>::new(world);
        planner.add_system::<SerializeSystem>(SerializeSystem, "serialize_system", 0);

        planner.dispatch(());
        planner.wait();
    }
}

#[cfg(not(feature="serialize"))]
mod s {
    pub fn main_redirect() {
        println!("This example requires the feature \"serialize\" to be enabled.");
        println!("You can enable it temporarily with: ");
        println!("    cargo run --example serialize --features serialize");
    }
}

fn main() {
    s::main_redirect();
}
