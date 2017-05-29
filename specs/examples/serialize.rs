extern crate shred;
#[macro_use]
extern crate shred_derive;
extern crate specs;
#[macro_use]
extern crate specs_derive;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

fn main() {
    use serde::Serialize;
    use serde::de::DeserializeSeed;
    use serde_json::{Serializer, from_str as json_from_str};
    use specs::{Component, DispatcherBuilder, Entities, Join, PackedData, System, VecStorage,
                World, WriteStorage, WorldSerializer, WorldDeserializer};

    #[derive(Debug, Serialize, Deserialize)]
    struct CompSerialize {
        field: u32,
        other: bool,
    }
    impl Component for CompSerialize {
        type Storage = VecStorage<Self>;
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct CompFloat(f32);
    impl Component for CompFloat {
        type Storage = VecStorage<CompFloat>;
    }

    #[derive(ComponentGroup)]
    struct SerialGroup {
        #[group(serialize)]
        comp_serialize: CompSerialize,

        #[group(serialize)]
        comp_float: CompFloat,
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

    #[derive(SystemData)]
    struct RemovalData<'a> {
        entities: Entities<'a>,
        comp_serial: WriteStorage<'a, CompSerialize>,
        comp_float: WriteStorage<'a, CompFloat>,
    }

    struct RemovalSystem;
    impl<'a, C> System<'a, C> for RemovalSystem {
        type SystemData = RemovalData<'a>;

        fn work(&mut self, mut data: RemovalData, _: C) {
            // Remove all components
            for (entity, _) in (&*data.entities, &data.comp_serial.check()).join() {
                data.comp_serial.remove(entity);
            }
            for (entity, _) in (&*data.entities, &data.comp_float.check()).join() {
                data.comp_float.remove(entity);
            }
        }
    }

    let mut world = World::new();
    world.register_group::<SerialGroup>();

    world.create_entity()
        .with(CompFloat(2.71828182845))
        .build();
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
        .with(CompFloat(3.14159265358979))
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
        .with(CompFloat(5.0))
        .build();

    let mut dispatcher = DispatcherBuilder::new()
        .add(SerializeSystem, "ser", &[])
        .build();

    dispatcher.dispatch(&mut world.res);
    world.maintain();

    let serialized = {
        let world_serializer = WorldSerializer::<SerialGroup>::new(&world);
        let serialized = serde_json::to_string_pretty(&world_serializer).unwrap();
        println!("{}", serialized);
        serialized
    };

    {
        let mut dispatcher = DispatcherBuilder::new()
            .add(RemovalSystem, "removal", &[])
            .build();

        dispatcher.dispatch(&mut world.res, ());
    }

    {
        let world_serializer = WorldSerializer::<SerialGroup>::new(&world);
        let serialized = serde_json::to_string_pretty(&world_serializer).unwrap();
        println!("before: {}", serialized);
    }

    {
        let entity_list: Vec<_> = {
            let entities = world.read_resource::<specs::Entities>();
            entities.join().collect()
        };

        let world_deserializer = WorldDeserializer::<SerialGroup>::new(&mut world, entity_list.as_slice());
        let mut json_deserializer = serde_json::Deserializer::from_str(&serialized);
        world_deserializer.deserialize(&mut json_deserializer);
    }

    {
        let world_serializer = WorldSerializer::<SerialGroup>::new(&world);
        let serialized = serde_json::to_string_pretty(&world_serializer).unwrap();
        println!("after: {}", serialized);
    }
}
