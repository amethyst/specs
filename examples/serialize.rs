#[macro_use]
extern crate serde;
extern crate serde_json;
extern crate shred;
#[macro_use]
extern crate shred_derive;
extern crate specs;

fn main() {
    use serde::Serialize;
    use serde_json::{from_str as json_from_str, Serializer};
    use specs::{Component, DispatcherBuilder, Entities, Join, PackedData, System, VecStorage,
                World, WriteStorage};

    #[derive(Debug, Serialize, Deserialize)]
    struct CompSerialize {
        field: u32,
        other: bool,
    }
    impl Component for CompSerialize {
        type Storage = VecStorage<Self>;
    }

    #[derive(SystemData)]
    struct Data<'a> {
        entities: Entities<'a>,
        comp: WriteStorage<'a, CompSerialize>,
    }

    struct SerializeSystem;

    impl<'a> System<'a> for SerializeSystem {
        type SystemData = Data<'a>;

        fn run(&mut self, mut data: Data) {
            // Serialize the storage into JSON
            let mut serializer = Serializer::pretty(Vec::new());
            data.comp.serialize(&mut serializer).unwrap();

            let serialized = serializer
                .into_inner()
                .iter()
                .map(|b| *b as char)
                .collect::<String>();
            println!("Serialized storage: {}", serialized);

            // Get a list of all entities in the world
            let entity_list: Vec<_> = (&*data.entities).join().collect();

            // Remove all components
            for (entity, _) in (&*data.entities, &data.comp.check()).join() {
                data.comp.remove(entity);
            }

            // Deserialize with entity list
            let list: PackedData<CompSerialize> = json_from_str(&serialized).unwrap();
            println!("list: {:?}", list);

            data.comp.merge(entity_list.as_slice(), list).unwrap();

            for (entity, _) in (&*data.entities, &data.comp.check()).join() {
                println!("Has: {:?}", entity);
            }
        }
    }

    let mut world = World::new();
    world.register::<CompSerialize>();

    world.create_entity().build();
    world.create_entity().build();
    world
        .create_entity()
        .with(CompSerialize {
            field: 5,
            other: true,
        })
        .build();
    world.create_entity().build();
    world.create_entity().build();
    world
        .create_entity()
        .with(CompSerialize {
            field: 5,
            other: true,
        })
        .build();
    world
        .create_entity()
        .with(CompSerialize {
            field: 10,
            other: false,
        })
        .build();
    world.create_entity().build();
    world
        .create_entity()
        .with(CompSerialize {
            field: 0,
            other: false,
        })
        .build();

    let mut dispatcher = DispatcherBuilder::new()
        .add(SerializeSystem, "ser", &[])
        .build();

    dispatcher.dispatch(&mut world.res);
    world.maintain();
}
