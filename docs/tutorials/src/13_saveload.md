# Saveload

`saveload` is a module that provides mechanisms to serialize and deserialize a
`World`, it makes use of the popular [`serde`] library and requires the feature
flag `serde` to be enabled for `specs` in your `Cargo.toml` file.

At a high level, it works by defining a `Marker` component as well as a
`MarkerAllocator` resource. Marked entities will be the only ones subject to
serialization and deserialization.

`saveload` also defines `SerializeComponents` and `DeserializeComponents`,
these do the heavy lifting of exporting and importing.

Let's go over everything, point by point:

## `Marker` and `MarkerAllocator`

`Marker` and `MarkerAllocator<M: Marker>` are actually traits, simple
implementations are available with `SimpleMarker<T: ?Sized>` and
`SimpleMarkerAllocator<T: ?Sized>`, which you may use multiple times with
[Zero Sized Types].

```rust,ignore
struct NetworkSync;
struct FilePersistent;

fn main() {
    let mut world = World::new();

    world.register::<SimpleMarker<NetworkSync>>();
    world.insert(SimpleMarkerAllocator::<NetworkSync>::default());

    world.register::<SimpleMarker<FilePersistent>>();
    world.insert(SimpleMarkerAllocator::<FilePersistent>::default());

    world
        .create_entity()
        .marked::<SimpleMarker<NetworkSync>>()
        .marked::<SimpleMarker<FilePersistent>>()
        .build();
}

```

You may also roll your own implementations like so:

```rust,ignore
use specs::{prelude::*, saveload::{MarkedBuilder, Marker, MarkerAllocator}};

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
struct MyMarker(u64);

impl Component for MyMarker {
    type Storage = VecStorage<Self>;
}

impl Marker for MyMarker {
    type Identifier = u64;
    type Allocator = MyMarkerAllocator;

    fn id(&self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct MyMarkerAllocator(std::collections::HashMap<u64, Entity>);

impl MarkerAllocator<MyMarker> for MyMarkerAllocator {
    fn allocate(&mut self, entity: Entity, id: Option<u64>) -> MyMarker {
        let id = id.unwrap_or_else(|| self.unused_key()));
        self.0.insert(id, entity);
        MyMarker(id)
    }

    fn retrieve_entity_internal(&self, id: u64) -> Option<Entity> {
        self.0.get(&id).cloned()
    }

    fn maintain(
        &mut self,
        entities: &EntitiesRes,
        storage: &ReadStorage<MyMarker>,
    ) {
        // naive and possibly costly implementation, the techniques in
        // chapter 12 would be useful here!
        self.0 = (entities, storage)
            .join()
            .map(|(entity, marker)| (marker.0, entity))
            .collect();
    }
}

fn main() {
    let mut world = World::new();

    world.register::<MyMarker>();
    world.insert(MyMarkerAllocator::default());

    world
        .create_entity()
        .marked::<MyMarker>()
        .build();
}
```

Note that the trait `MarkedBuilder` must be imported to mark entities during
creation, it is implemented for `EntityBuilder` and `LazyBuilder`. Marking an
entity that is already present is straightforward:

```rust,ignore
fn mark_entity(
    entity: Entity,
    mut allocator: Write<SimpleMarkerAllocator<A>>,
    mut storage: WriteStorage<SimpleMarker<A>>,
) {
    use MarkerAllocator; // for MarkerAllocator::mark

    match allocator.mark(entity, &mut storage) {
        None => println!("entity was dead before it could be marked"),
        Some((_, false)) => println!("entity was already marked"),
        Some((_, true)) => println!("entity successfully marked"),
    }
}
```

## Serialization and Deserialization

As previously mentioned, `SerializeComponents` and `DeserializeComponents` are
the two heavy lifters. They're traits as well, however, that's just an
implementation detail, they are used like functions. They're implemented over
tuples of up to 16 `ReadStorage`/`WriteStorage`.

Here is an example showing how to serialize:

```rust,ignore
specs::saveload::SerializeComponents
    ::<Infallible, SimpleMarker<A>>
    ::serialize(
        &(position_storage, mass_storage),      // tuple of ReadStorage<'a, _>
        &entities,                              // Entities<'a>
        &marker_storage,                        // ReadStorage<'a, SimpleMarker<A>>
        &mut serializer,                        // serde::Serializer
    )   // returns Result<Serializer::Ok, Serializer::Error>
```

and now, how to deserialize:

```rust,ignore
specs::saveload::DeserializeComponents
    ::<Infallible, SimpleMarker<A>>
    ::deserialize(
        &mut (position_storage, mass_storage),  // tuple of WriteStorage<'a, _>
        &entities,                              // Entities<'a>
        &mut marker_storage,                    // WriteStorage<'a SimpleMarker<A>>
        &mut marker_allocator,                  // Write<'a, SimpleMarkerAllocator<A>>
        &mut deserializer,                      // serde::Deserializer
    )   // returns Result<(), Deserializer::Error>
```

As you can see, all parameters but one are `SystemData`, the easiest way to
access those would be through systems ([chapter on this subject][c6]) or by
calling `World::system_data`:

```rust,ignore
let (
    entities,
    mut marker_storage,
    mut marker_allocator,
    mut position_storage,
    mut mass_storage,
) = world.system_data::<(
    Entities,
    WriteStorage<SimpleMarker<A>>,
    Write<SimpleMarkerAllocator<A>>,
    WriteStorage<Position>,
    WriteStorage<Mass>,
)>();
```

Each `Component` that you will read from and write to must implement
`ConvertSaveload`, it's a benign trait and is implemented for all `Component`s
that are `Clone + serde::Serialize + serde::DeserializeOwned`, however you may
need to implement it (or derive it using [`specs-derive`]). In which case, you
may introduce more bounds to the first generic parameter and will need to
replace `Infallible` with a custom type, this custom type must implement
`From<<TheComponent as ConvertSaveload>::Error>` for all `Component`s,
basically.

[Zero Sized Types]: https://doc.rust-lang.org/nomicon/exotic-sizes.html#zero-sized-types-zsts
[`newtype`]: https://doc.rust-lang.org/1.0.0/style/features/types/newtype.html
[`serde`]: https://docs.rs/serde
[`specs-derive`]: https://docs.rs/specs-derive
[c6]: ./06_system_data.html
