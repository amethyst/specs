extern crate ron;
#[macro_use]
extern crate serde;
extern crate specs;

use specs::{Component, RunNow, System, VecStorage, World};
use specs::error::NoError;
use specs::saveload::{U64Marker, U64MarkerAllocator, WorldDeserialize,
                      WorldSerialize};

const ENTITIES: &str = "
[
    (
        marker: (0),
        components: (
            Some((
                x: 10,
                y: 20,
            )),
            Some((30.5)),
        ),
    ),
    (
        marker: (1),
        components: (
            Some(Pos(
                x: 5,
                y: 2,
            )),
            None,
        ),
    ),
]
";

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct Pos {
    x: f32,
    y: f32,
}

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct Mass(f32);

impl Component for Mass {
    type Storage = VecStorage<Self>;
}

fn main() {
    use specs::Join;

    let mut world = World::new();

    world.register::<Pos>();
    world.register::<Mass>();
    world.register::<U64Marker>();

    world.add_resource(U64MarkerAllocator::new());

    world
        .create_entity()
        .with(Pos { x: 1.0, y: 2.0 })
        .with(Mass(0.5))
        .marked::<U64Marker>()
        .build();

    world
        .create_entity()
        .with(Pos { x: 7.0, y: 2.0 })
        .with(Mass(4.5))
        .marked::<U64Marker>()
        .build();

    struct Serialize;

    impl<'a> System<'a> for Serialize {
        type SystemData = WorldSerialize<'a, U64Marker, NoError, (Pos, Mass)>;

        fn run(&mut self, mut world: Self::SystemData) {
            let s = ron::ser::pretty::to_string(&world).unwrap();

            println!("{}", s);

            world.remove_serialized();
        }
    }

    Serialize.run_now(&world.res);

    // -----------------

    struct Deserialize;

    impl<'a> System<'a> for Deserialize {
        type SystemData = WorldDeserialize<'a, U64Marker, NoError, (Pos, Mass)>;

        fn run(&mut self, world: Self::SystemData) {
            use ron::de::Deserializer;
            use serde::de::DeserializeSeed;

            let mut de = Deserializer::from_str(ENTITIES);
            world.deserialize(&mut de).unwrap();
        }
    }

    Deserialize.run_now(&world.res);

    println!("{:#?}", (&world.read::<Pos>()).join().collect::<Vec<_>>());
}
