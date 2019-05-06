# Saveload

`saveload` is a module that provides mechanisms to serialize and deserialize a `World`, it makes use of the popular [`serde`] library and requires the feature flag `serde` to be enabled for `specs` in your `Cargo.toml` file.

At a high level, it works by defining an identifier `Component`, the `Marker`, as well as a `Resource`, the `MarkerAllocator`. Marked entities will be the only ones subject to serialization and deserialization.

`saveload` also defines `SerializeComponents` and `DeserializeComponents`, these do the heavy lifting of exporting and importing, as such, they take a bunch of `SystemData` as parameters.

Let's go over everything, point by point:

## `Marker` and `MarkerAllocator`

`Marker` and `MarkerAllocator<M: Marker>` are actually traits, a simple implementation is available with `U64Marker` and `U64MarkerAllocator`. While these work well, you may want use multiple `Marker` types (one for persistent storage, another for networked synchronization, ...), in which case, you could simply implement the traits over multiple [`newtype`]s or implement the trait once over a generic type. Let's see what the latter solution would look like in code:

```rust,ignore
#[derive(Deserialize, Serialize)] // from serde
struct MyMarker<T> {
    id: u64,
    _phantom: std::marker::PhantomData<T>,
}

// Note: The following trait implementations were skipped:
// Clone, Debug, Hash, Eq, PartialEq, specs::Component, specs::saveload::Marker
// The code for Marker was simply ripped from the U64Marker implementation
// The traits from std had to be manually implemented because the PhantomData
// prevents deriving.

struct MyMarkerAllocator<T> {
    index: u64,
    mapping: std::collections::HashMap<u64, Entity>,
    _phantom: std::marker::PhantomData<T>,
}

// Note: The following trait implementations were skipped:
// Default, specs::saveload::MarkerAllocator
// The code for MarkerAllocator was simply ripped from the U64MarkerAllocator
// implementation

struct FileStored;
struct Networked;

fn main() {
    use specs::prelude::*;
    use specs::saveload::MarkedBuilder;

    let mut world = World::new();

    world.register::<MyMarker<FileStored>>();
    world.register::<MyMarker<Networked>>();

    world.add_resource(MyMarkerAllocator::<FileStored>::default());
    world.add_resource(MyMarkerAllocator::<Networked>::default());

    world
        .create_entity()
        .marked::<MyMarker<FileStored>>()
        .marked::<MyMarker<Networked>>()
        .build();
}
```

Note that the trait `MarkedBuilder` must be imported to mark entities, it is
implemented for `EntityBuilder` and `LazyBuilder`.

## Serialization and Deserialization

As previously mentioned, `SerializeComponents` and `DeserializeComponents` are
the two heavy lifters. They're traits as well and they're implemented over
tuples of up to 16 `ReadStorage`/`WriteStorage`.

Since the call sites are very busy, I'll do my best to make it understandable
by exploding them, here goes serializing:

```rust,ignore
specs::saveload::SerializeComponents
    ::<NoError, U64Marker>
    ::serialize(
        &(&position_storage, &mass_storage),    // tuple of ReadStorage<'a, _>
        &entities,                              // Entities<'a>
        &marker_storage,                        // ReadStorage<'a, U64Marker>
        &mut serializer,                        // serde::Serializer
    )   // returns Result<Serializer::Ok, Serializer::Error>
```

and deserializing:

```rust,ignore
specs::saveload::DeserializeComponents
    ::<NoError, U64Marker>
    ::deserialize(
        &mut (position_storage, mass_storage),  // tuple of WriteStorage<'a, _>
        &entities,                              // Entities<'a>
        &mut marker_storage,                    // WriteStorage<'a U64Marker>
        &mut marker_allocator,                  // Write<'a, U64MarkerAllocator>
        &mut deserializer,                      // serde::Deserializer
    )   // returns Result<(), Deserializer::Error>
```

It should be obvious from all their `SystemData` parameters that they are best
used in `System`s.

Each `Component` that you will read from and write to must implement
`ConvertSaveload`, it's a benign trait and is implemented for all `Component`s
that are `Clone + serde::Serialize + serde::DeserializeOwned`, however you may
need to implement it (or derive it using [`specs-derive`]). In which case, you
may introduce more bounds to the first generic parameter and will need to
replace `NoError` with a custom type, this custom type must implement
`From<ConvertSaveload::Error>` for all `Component`s, basically.

[`newtype`]: https://doc.rust-lang.org/1.0.0/style/features/types/newtype.html
[`serde`]: https://docs.rs/serde
[`specs-derive`]: https://docs.rs/specs-derive
