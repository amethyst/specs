extern crate ron;
#[macro_use]
extern crate serde;
extern crate specs;

use specs::{Component, Entities, RunNow, ReadStorage, System, VecStorage, World, WriteStorage};
use specs::saveload::{DeserializeComponents, SerializeComponents, U64Marker, U64MarkerAllocator};

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

enum Combined {
    Ron(ron::ser::Error),
}

impl From<ron::ser::Error> for Combined {
    fn from(x: ron::ser::Error) -> Self {
        Combined::Ron(x)
    }
}

impl From<()> for Combined {
    fn from(_: ()) -> Self {
        unimplemented!()
    }
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
        type SystemData = (Entities<'a>, ReadStorage<'a, Pos>, ReadStorage<'a, Mass>, ReadStorage<'a, U64Marker>);

        fn run(&mut self, (ents, pos, vel, markers): Self::SystemData) {
            let mut ser = ron::ser::Serializer::new(Some(Default::default()), true);
            SerializeComponents::<Combined, _>::serialize(
                &(&pos, &vel), &ents, &markers, &mut ser
            );
            // TODO: Specs should return an error which combines serialization
            // and component errors.

            println!("{}", ser.into_output_string());
        }
    }

    Serialize.run_now(&world.res);

    // -----------------

//    struct Deserialize;
//
//    impl<'a> System<'a> for Deserialize {
//        type SystemData = WorldDeserialize<'a, U64Marker, NoError, (Pos, Mass)>;
//
//        fn run(&mut self, world: Self::SystemData) {
//            use ron::de::Deserializer;
//            use serde::de::DeserializeSeed;
//
//            let mut de = Deserializer::from_str(ENTITIES);
//            world.deserialize(&mut de).unwrap();
//        }
//    }
//
//    Deserialize.run_now(&world.res);

    println!("{:#?}", (&world.read::<Pos>()).join().collect::<Vec<_>>());
}
