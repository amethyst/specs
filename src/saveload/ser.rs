use std::fmt::Display;
use std::marker::PhantomData;

use serde::ser::{self, Serialize, SerializeSeq, Serializer};

use {Entities, FetchMut, Join, ReadStorage, WriteStorage};

use saveload::{Components, EntityData, Storages};
use saveload::marker::{Marker, MarkerAllocator};

/// Serialize components from specified storages via `SerializableComponent::save`
/// of all marked entities with provided serializer.
/// When the component gets serialized with `SerializableComponent::save` method
/// the closure passed in `ids` argument marks unmarked `Entity` (the marker of which was requested)
/// and it will get serialized recursively.
/// For serializing without such recursion see `serialize` function.
pub fn serialize_recursive<'a, M, E, T, S>(
    entities: &Entities<'a>,
    storages: &<T as Storages<'a>>::ReadStorages,
    markers: &mut WriteStorage<'a, M>,
    allocator: &mut FetchMut<'a, M::Allocator>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    M: Marker,
    E: Display,
    T: Components<M::Identifier, E>,
    S: Serializer,
{
    let mut serseq = serializer.serialize_seq(None)?;
    let mut to_serialize = (&**entities, &*markers)
        .join()
        .map(|(e, m)| (e, *m))
        .collect::<Vec<_>>();
    while !to_serialize.is_empty() {
        let mut add = vec![];
        {
            let mut ids = |entity| -> Option<M::Identifier> {
                let (marker, added) = allocator.mark(entity, markers);
                if added {
                    add.push((entity, marker));
                }
                Some(marker.id())
            };
            for (entity, marker) in to_serialize {
                serseq.serialize_element(&EntityData::<M, E, T> {
                    marker,
                    components: T::save(entity, storages, &mut ids).map_err(
                        ser::Error::custom,
                    )?,
                })?;
            }
        }
        to_serialize = add;
    }
    serseq.end()
}


/// Serialize components from specified storages via `SerializableComponent::save`
/// of all marked entities with provided serializer.
/// When the component gets serialized with `SerializableComponent::save` method
/// the closure passed in `ids` arguemnt returns `None` for unmarked `Entity`.
/// In this case `SerializableComponent::save` may perform workaround (forget about `Entity`)
/// or fail.
/// So the function doesn't recursively mark referenced entities.
/// For recursive marking see `serialize_recursive`
pub fn serialize<'a, M, E, T, S>(
    entities: &Entities<'a>,
    storages: &<T as Storages<'a>>::ReadStorages,
    markers: &ReadStorage<'a, M>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    M: Marker,
    E: Display,
    T: Components<M::Identifier, E>,
    S: Serializer,
{
    let mut serseq = serializer.serialize_seq(None)?;
    let ids = |entity| -> Option<M::Identifier> { markers.get(entity).map(Marker::id) };
    for (entity, marker) in (&**entities, &*markers).join() {
        serseq.serialize_element(&EntityData::<M, E, T> {
            marker: *marker,
            components: T::save(entity, storages, &ids).map_err(ser::Error::custom)?,
        })?;
    }
    serseq.end()
}

/// This type implements `Serialize` so that it may be used in generic environment
/// where `Serialize` implementation is expected.
/// It may be constructed manually with `WorldSerialize::new`.
/// Or fetched from `System` as `SystemData`.
/// Serializes components in tuple `T` with marker `M`.
#[derive(SystemData)]
pub struct WorldSerialize<'a, M: Marker, E, T: Components<M::Identifier, E>> {
    entities: Entities<'a>,
    storages: <T as Storages<'a>>::ReadStorages,
    markers: ReadStorage<'a, M>,
    pd: PhantomData<E>,
}

macro_rules! world_serialize_new_functions {
    ($($a:ident),*) => {
        impl<'a, X, Z $(,$a)*> WorldSerialize<'a, X, Z, ($($a,)*)>
            where X: Marker,
                $(
                    $a: super::SaveLoadComponent<X::Identifier>,
                    Z: From<$a::Error>,
                )*
        {
            /// Create serializable structure from storages
            #[allow(non_snake_case)]
            pub fn new(entities: Entities<'a>,
                       markers: ReadStorage<'a, X>
                       $(,$a: ReadStorage<'a, $a>)*) -> Self
            {
                WorldSerialize {
                    entities,
                    storages: ($($a,)*),
                    markers,
                    pd: PhantomData,
                }
            }
        }

        world_serialize_new_functions!(@ $($a),*);
    };

    (@) => {};
    (@ $ah:ident $(,$a:ident)*) => {
        // Call again for tail
        world_serialize_new_functions!($($a),*);
    };
}

world_serialize_new_functions!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);

impl<'a, M, E, T> WorldSerialize<'a, M, E, T>
where
    M: Marker,
    T: Components<M::Identifier, E>,
{
    /// Remove all marked entities
    /// Use this if you want to delete entities that were just serialized
    pub fn remove_serialized(&mut self) {
        for (entity, _) in (&*self.entities, &self.markers.check()).join() {
            let _ = self.entities.delete(entity);
        }
    }
}

impl<'a, M, E, T> Serialize for WorldSerialize<'a, M, E, T>
where
    M: Marker,
    E: Display,
    T: Components<M::Identifier, E>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize::<M, E, T, S>(&self.entities, &self.storages, &self.markers, serializer)
    }
}
