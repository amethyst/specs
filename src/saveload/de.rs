use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;

use serde::de::{self, Deserialize, DeserializeSeed, Deserializer, SeqAccess, Visitor};

use saveload::{Components, EntityData, Storages};
use saveload::marker::{Marker, MarkerAllocator};
use shred::Write;
use storage::WriteStorage;
use world::Entities;

/// Wrapper for `Entity` and tuple of `WriteStorage`s that implements `serde::Deserialize`.
struct DeserializeEntity<'a, 'b: 'a, M: Marker, E, T: Components<M::Identifier, E>> {
    entities: &'a Entities<'b>,
    storages: &'a mut <T as Storages<'b>>::WriteStorages,
    markers: &'a mut WriteStorage<'b, M>,
    allocator: &'a mut Write<'b, M::Allocator>,
    pd: PhantomData<(E, T)>,
}

impl<'de, 'a, 'b: 'a, M, E, T> DeserializeSeed<'de> for DeserializeEntity<'a, 'b, M, E, T>
where
    M: Marker,
    E: Display,
    T: Components<M::Identifier, E>,
{
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        let DeserializeEntity {
            entities,
            storages,
            markers,
            allocator,
            ..
        } = self;
        let data = EntityData::<M, E, T>::deserialize(deserializer)?;
        let entity = allocator.get_marked(data.marker.id(), entities, markers);
        markers
            .get_mut(entity)
            .ok_or("Allocator is broken")
            .map_err(de::Error::custom)?
            .update(data.marker);
        let ids = |marker: M::Identifier| Some(allocator.get_marked(marker, entities, markers));

        match T::load(entity, data.components, storages, ids) {
            Ok(()) => Ok(()),
            Err(err) => Err(de::Error::custom(err)),
        }
    }
}

/// Wrapper for `Entities` and tuple of `WriteStorage`s that implements `serde::de::Visitor`
struct VisitEntities<'a, 'b: 'a, M: Marker, E, T: Components<M::Identifier, E>> {
    entities: &'a Entities<'b>,
    storages: &'a mut <T as Storages<'b>>::WriteStorages,
    markers: &'a mut WriteStorage<'b, M>,
    allocator: &'a mut Write<'b, M::Allocator>,
    pd: PhantomData<(E, T)>,
}

impl<'de, 'a, 'b: 'a, M, E, T> Visitor<'de> for VisitEntities<'a, 'b, M, E, T>
where
    M: Marker,
    E: Display,
    T: Components<M::Identifier, E>,
{
    type Value = ();

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "Sequence of serialized entities")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
    where
        A: SeqAccess<'de>,
    {
        while seq.next_element_seed(DeserializeEntity {
            entities: self.entities,
            storages: self.storages,
            markers: self.markers,
            allocator: self.allocator,
            pd: self.pd,
        })?
            .is_some()
        {}

        Ok(())
    }
}

/// Deserialize entities according to markers.
pub fn deserialize<'a, 'de, D, M, E, T>(
    entities: &Entities<'a>,
    storages: &mut <T as Storages<'a>>::WriteStorages,
    markers: &mut WriteStorage<'a, M>,
    allocator: &mut Write<'a, M::Allocator>,
    deserializer: D,
) -> Result<(), D::Error>
where
    M: Marker,
    E: Display,
    T: Components<M::Identifier, E>,
    D: Deserializer<'de>,
{
    deserializer.deserialize_seq(VisitEntities::<M, E, T> {
        entities,
        storages,
        markers,
        allocator,
        pd: PhantomData,
    })
}

/// Struct which implements `DeserializeSeed` to allow serializing
/// components from `World`.
#[derive(SystemData)]
pub struct WorldDeserialize<'a, M: Marker, E, T: Components<M::Identifier, E>> {
    entities: Entities<'a>,
    storages: <T as Storages<'a>>::WriteStorages,
    markers: WriteStorage<'a, M>,
    allocator: Write<'a, M::Allocator>,
    pd: PhantomData<E>,
}

impl<'de, 'a, M, E, T> DeserializeSeed<'de> for WorldDeserialize<'a, M, E, T>
where
    M: Marker,
    E: Display,
    T: Components<M::Identifier, E>,
{
    type Value = ();

    fn deserialize<D>(mut self, deserializer: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize::<D, M, E, T>(
            &mut self.entities,
            &mut self.storages,
            &mut self.markers,
            &mut self.allocator,
            deserializer,
        )
    }
}
