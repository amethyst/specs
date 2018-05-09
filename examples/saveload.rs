extern crate ron;
#[macro_use]
extern crate serde;
extern crate specs;

use std::fmt;

use specs::error::NoError;
use specs::prelude::*;
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

#[derive(Debug)]
enum Combined {
    Ron(ron::ser::Error),
}

impl fmt::Display for Combined {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Combined::Ron(ref e) => write!(f, "{}", e),
        }
    }
}

impl From<ron::ser::Error> for Combined {
    fn from(x: ron::ser::Error) -> Self {
        Combined::Ron(x)
    }
}

impl From<NoError> for Combined {
    fn from(e: NoError) -> Self {
        match e {}
    }
}

fn main() {
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
        type SystemData = (
            Entities<'a>,
            ReadStorage<'a, Pos>,
            ReadStorage<'a, Mass>,
            ReadStorage<'a, U64Marker>,
        );

        fn run(&mut self, (ents, pos, mass, markers): Self::SystemData) {
            let mut ser = ron::ser::Serializer::new(Some(Default::default()), true);
            SerializeComponents::<NoError, U64Marker>::serialize(
                &(&pos, &mass),
                &ents,
                &markers,
                &mut ser,
            ).unwrap_or_else(|e| eprintln!("Error: {}", e));
            // TODO: Specs should return an error which combines serialization
            // and component errors.

            println!("{}", ser.into_output_string());
        }
    }

    Serialize.run_now(&world.res);

    // -----------------

    struct Deserialize;

    impl<'a> System<'a> for Deserialize {
        type SystemData = (
            Entities<'a>,
            Write<'a, U64MarkerAllocator>,
            WriteStorage<'a, Pos>,
            WriteStorage<'a, Mass>,
            WriteStorage<'a, U64Marker>,
        );

        fn run(&mut self, (ent, mut alloc, pos, mass, mut markers): Self::SystemData) {
            use ron::de::Deserializer;

            if let Ok(mut de) = Deserializer::from_str(ENTITIES) {
                DeserializeComponents::<Combined, _>::deserialize(
                    &mut (pos, mass),
                    &ent,
                    &mut markers,
                    &mut alloc,
                    &mut de,
                ).unwrap_or_else(|e| eprintln!("Error: {}", e));
            }
        }
    }

    Deserialize.run_now(&world.res);

    println!(
        "{:#?}",
        (&world.read_storage::<Pos>()).join().collect::<Vec<_>>()
    );
}
