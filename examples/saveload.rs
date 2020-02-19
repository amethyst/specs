extern crate ron;
#[macro_use]
extern crate serde;
extern crate specs;

use std::{convert::Infallible, fmt};

use specs::{
    prelude::*,
    saveload::{
        DeserializeComponents, MarkedBuilder, SerializeComponents, SimpleMarker,
        SimpleMarkerAllocator,
    },
};

// This is an example of how the serialized data of two entities might look on
// disk.
//
// When serializing entities, they are written in an array of tuples, each tuple
// representing one entity. The entity's marker and components are written as
// fields into these tuples, knowing nothing about the original entity's id.
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

// A dummy component that can be serialized and deserialized.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct Pos {
    x: f32,
    y: f32,
}

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

// A dummy component that can be serialized and deserialized.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct Mass(f32);

impl Component for Mass {
    type Storage = VecStorage<Self>;
}

// It is necessary to supply the `(De)SerializeComponents`-trait with an error
// type that implements the `Display`-trait. In this case we want to be able to
// return different errors, and we are going to use a `.ron`-file to store our
// data. Therefore we use a custom enum, which can display both the
// `Infallible`and `ron::ser::Error` type. This enum could be extended to
// incorporate for example `std::io::Error` and more.
#[derive(Debug)]
enum Combined {
    Ron(ron::ser::Error),
}

// Implementing the required `Display`-trait, by matching the `Combined` enum,
// allowing different error types to be displayed.
impl fmt::Display for Combined {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Combined::Ron(ref e) => write!(f, "{}", e),
        }
    }
}

// This returns the `ron::ser:Error` in form of the `Combined` enum, which can
// then be matched and displayed accordingly.
impl From<ron::ser::Error> for Combined {
    fn from(x: ron::ser::Error) -> Self {
        Combined::Ron(x)
    }
}

// This cannot be called.
impl From<Infallible> for Combined {
    fn from(e: Infallible) -> Self {
        match e {}
    }
}

struct NetworkSync;

fn main() {
    let mut world = World::new();

    // Since in this example no system uses these resources, they have to be
    // registered manually. This is typically not required.
    world.register::<Pos>();
    world.register::<Mass>();
    world.register::<SimpleMarker<NetworkSync>>();

    // Adds a predefined marker allocator to the world, as a resource.
    // This predifined marker uses a `HashMap<u64, Entity>` to keep track of all
    // entities that should be (de)serializable, as well as which ids are
    // already in use.
    world.insert(SimpleMarkerAllocator::<NetworkSync>::new());

    world
        .create_entity()
        .with(Pos { x: 1.0, y: 2.0 })
        .with(Mass(0.5))
        // The `.marked` function belongs to the [`MarkedBuilder`](struct.MarkedBuilder.html) trait,
        // which is implemented for example for the [`EntityBuilder`](struct.EntityBuilder.html).
        // It yields the next higher id, that is not yet in use.
        //
        // Since the `Marker` is passed as a generic type parameter, it is possible to use several different `MarkerAllocators`,
        // e.g. to keep track of different types of entities, with different ids.
        // **Careful when deserializing, it is not always clear for every fileforamt whether a number is supposed to be i.e. a `u32` or `u64`!**
        .marked::<SimpleMarker<NetworkSync>>()
        .build();

    world
        .create_entity()
        .with(Pos { x: 7.0, y: 2.0 })
        .with(Mass(4.5))
        .marked::<SimpleMarker<NetworkSync>>()
        .build();

    // Here we create a system that lets us access the entities to serialize.
    struct Serialize;

    impl<'a> System<'a> for Serialize {
        // This SystemData contains the entity-resource, as well as all components that
        // shall be serialized, plus the marker component storage.
        type SystemData = (
            Entities<'a>,
            ReadStorage<'a, Pos>,
            ReadStorage<'a, Mass>,
            ReadStorage<'a, SimpleMarker<NetworkSync>>,
        );

        fn run(&mut self, (ents, pos, mass, markers): Self::SystemData) {
            // First we need a serializer for the format of choice, in this case the
            // `.ron`-format.
            let mut ser = ron::ser::Serializer::new(Some(Default::default()), true);

            // For serialization we use the
            // [`SerializeComponents`](struct.SerializeComponents.html)-trait's `serialize`
            // function. It takes two generic parameters:
            // * An unbound type -> `Infallible` (However, the serialize function expects it
            //   to be bound by the `Display`-trait)
            // * A type implementing the `Marker`-trait ->
            //   [SimpleMarker](struct.SimpleMarker.html) (a convenient, predefined marker)
            //
            // The first parameter resembles the `.join()` syntax from other specs-systems,
            // every component that should be serialized has to be put inside a tuple.
            //
            // The second and third parameters are just the entity-storage and
            // marker-storage, which get `.join()`ed internally.
            //
            // Lastly, we provide a mutable reference to the serializer of choice, which has
            // to have the `serde::ser::Serializer`-trait implemented.
            SerializeComponents::<Infallible, SimpleMarker<NetworkSync>>::serialize(
                &(&pos, &mass),
                &ents,
                &markers,
                &mut ser,
            )
            .unwrap_or_else(|e| eprintln!("Error: {}", e));
            // TODO: Specs should return an error which combines serialization
            // and component errors.

            // At this point, `ser` could be used to write its contents to a file, which is
            // not done here. Instead we print the content of this pseudo-file.
            println!("{}", ser.into_output_string());
        }
    }

    // Running the system results in a print to the standard output channel, in
    // `.ron`-format, showing how the serialized dummy entities look like.
    Serialize.run_now(&world);

    // -----------------

    // Just like the previous Serialize-system, we write a Deserialize-system.
    struct Deserialize;

    impl<'a> System<'a> for Deserialize {
        // This requires all the component storages our serialized entities have,
        // mutably, plus a `MarkerAllocator` resource to write the deserialized
        // ids into, so that we can later serialize again.
        type SystemData = (
            Entities<'a>,
            Write<'a, SimpleMarkerAllocator<NetworkSync>>,
            WriteStorage<'a, Pos>,
            WriteStorage<'a, Mass>,
            WriteStorage<'a, SimpleMarker<NetworkSync>>,
        );

        fn run(&mut self, (ent, mut alloc, pos, mass, mut markers): Self::SystemData) {
            // The `const ENTITIES: &str` at the top of this file was formatted according to
            // the `.ron`-specs, therefore we need a `.ron`-deserializer.
            // Others can be used, as long as they implement the
            // `serde::de::Deserializer`-trait.
            use ron::de::Deserializer;

            // Typical file operations are omitted in this example, since we do not have a
            // seperate file, but a `const &str`. We use a convencience function
            // of the `ron`-crate: `from_str`, to convert our data form the top of the file.
            if let Ok(mut de) = Deserializer::from_str(ENTITIES) {
                // Again, we need to pass in a type implementing the `Display`-trait,
                // as well as a type implementing the `Marker`-trait.
                // However, from the function parameter `&mut markers`, which refers to the
                // `SimpleMarker`-storage, the necessary type of marker can be
                // inferred, hence the `, _>´.
                DeserializeComponents::<Combined, _>::deserialize(
                    &mut (pos, mass),
                    &ent,
                    &mut markers,
                    &mut alloc,
                    &mut de,
                )
                .unwrap_or_else(|e| eprintln!("Error: {}", e));
            }
        }
    }

    // If we run this system now, the `ENTITIES: &str` is going to be deserialized,
    // and two entities are created.
    Deserialize.run_now(&world);

    // Printing the `Pos`-component storage entries to show the result of
    // deserializing.
    println!(
        "{:#?}",
        (&world.read_storage::<Pos>()).join().collect::<Vec<_>>()
    );
}
